use crate::{pin_future, ThreadPool};

use super::{notifier::Notifier, task::Task, task_queue::TaskQueue};

use cooked_waker::IntoWaker;
use parking_lot::{lock_api::MutexGuard, Condvar, Mutex, RawMutex};

use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

#[derive(Clone)]
pub struct Executor {
    cancel: Arc<AtomicBool>,
    lock_pair: Arc<(Mutex<bool>, Condvar)>,
    pool: Arc<ThreadPool>,
    queue: TaskQueue,
    started: Arc<AtomicBool>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        let result: Executor = Self {
            cancel: Arc::new(AtomicBool::new(false)),
            lock_pair: Arc::new((Mutex::new(false), Condvar::new())),
            pool: Arc::new(ThreadPool::default()),
            queue: TaskQueue::default(),
            started: Arc::new(AtomicBool::new(false)),
        };
        result.start();
        result
    }
}
impl Executor {
    fn load(&self) -> bool {
        self.started.load(Ordering::Acquire)
    }

    fn store(&self, val: bool) {
        self.started.store(val, Ordering::Release);
    }
}

impl Executor {
    pub(crate) fn spawn<Fut>(&self, task: Fut) -> Task
    where
        Fut: Future<Output = ()> + 'static + Send,
    {
        let task: Task = Task::new(task);
        self.queue.push(&task);

        if !self.load() {
            self.notify();
        }
        task
    }

    fn notify(&self) {
        self.store(true);
        let pair2: Arc<(Mutex<bool>, Condvar)> = self.lock_pair.clone();
        std::thread::spawn(move || {
            let (lock, cvar) = &*pair2;
            let mut started: MutexGuard<'_, RawMutex, bool> = lock.lock();
            *started = true;
            cvar.notify_one();
        });
    }

    pub(crate) fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
        *self.lock_pair.0.lock() = false;
        self.store(false);
        self.queue.drain_all();
        self.cancel.store(false, Ordering::Release);
    }

    pub(crate) fn run(&self) {
        while !self.cancel.load(Ordering::Acquire) {
            let queue = self.queue.clone();
            queue.for_each(|task| {
                let queue: TaskQueue = self.queue.clone();
                self.pool.submit(move || {
                    let waker: Waker = Arc::new(Notifier::default()).into_waker();
                    pin_future!(task);
                    let mut cx: Context<'_> = Context::from_waker(&waker);
                    match task.as_mut().poll(&mut cx) {
                        Poll::Ready(()) => return,
                        Poll::Pending => {
                            queue.push(&task);
                        }
                    }
                });
            });
        }
        self.poll_all();
        self.queue.drain_all();
    }

    fn poll_all(&self) {
        self.pool.wait_for_all();
    }

    pub(crate) fn start(&self) {
        let lock_pair: Arc<(Mutex<bool>, Condvar)> = self.lock_pair.clone();
        let executor: Executor = self.clone();
        std::thread::spawn(move || {
            let (lock, cvar) = &*lock_pair;
            let mut started: MutexGuard<'_, RawMutex, bool> = lock.lock();
            while !*started {
                cvar.wait(&mut started);
            }
            executor.run();
        });
    }
}
