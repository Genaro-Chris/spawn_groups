use crate::pin_future;

use super::{notifier::Notifier, task::Task, task_queue::TaskQueue};

use cooked_waker::IntoWaker;
use parking_lot::{lock_api::MutexGuard, Condvar, Mutex, RawMutex};
use threadpool::{Builder, ThreadPool};

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
    pool: ThreadPool,
    queue: TaskQueue,
    started: Arc<AtomicBool>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        let result: Executor = Self {
            cancel: Arc::new(AtomicBool::new(false)),
            lock_pair: Arc::new((Mutex::new(false), Condvar::new())),
            pool: Builder::new().thread_name("Executor".to_owned()).build(),
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
    pub(crate) fn spawn<Fut>(&mut self, task: Fut) -> Task
    where
        Fut: Future<Output = ()> + 'static + Send,
    {
        let task: Task = Task::new(task);
        self.queue.push(task.clone());

        if !self.load() {
            self.notify();
        }
        task
    }

    fn notify(&self) {
        self.store(true);
        let pair2: Arc<(Mutex<bool>, Condvar)> = self.lock_pair.clone();
        ThreadPool::new(1).execute(move || {
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
        while self.queue.clone().next().is_some() {}
        self.cancel.store(false, Ordering::Release);
    }

    pub(crate) fn run(&mut self) {
        loop {
            if !self.cancel.load(Ordering::Acquire) {
                for task in self.queue.clone() {
                    let queue: TaskQueue = self.queue.clone();
                    let notifier: Arc<Notifier> = Arc::new(Notifier::default());
                    self.pool.execute(move || {
                        let waker: Waker = notifier.clone().into_waker();
                        let task_clone: Task = task.clone();
                        pin_future!(task);
                        let mut cx: Context<'_> = Context::from_waker(&waker);
                        match task.as_mut().poll(&mut cx) {
                            Poll::Ready(()) => {}
                            Poll::Pending => {
                                queue.push(task_clone);
                            }
                        }
                    });
                }
            } else {
                self.poll_all();
                while self.queue.next().is_some() {}
                return;
            }
        }
    }

    fn poll_all(&self) {
        self.pool.join();
    }

    pub(crate) fn start(&self) {
        let lock_pair: Arc<(Mutex<bool>, Condvar)> = self.lock_pair.clone();
        let mut executor: Executor = self.clone();
        ThreadPool::new(1).execute(move || {
            let (lock, cvar) = &*lock_pair;
            let mut started: MutexGuard<'_, RawMutex, bool> = lock.lock();
            while !*started {
                cvar.wait(&mut started);
            }
            executor.run();
        });
    }
}
