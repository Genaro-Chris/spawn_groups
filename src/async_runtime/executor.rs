use super::{notifier::Notifier, task::Task, task_queue::TaskQueue};

use cooked_waker::IntoWaker;
use parking_lot::{Condvar, Mutex};
use threadpool::ThreadPool;

use std::{
    borrow::BorrowMut,
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::Context,
};

#[derive(Clone)]
pub struct Executor {
    cancel: Arc<AtomicBool>,
    lock_pair: Arc<(Mutex<bool>, Condvar)>,
    pool: Arc<ThreadPool>,
    queue: TaskQueue,
    started: Arc<AtomicBool>,
}

impl Default for Executor {
    fn default() -> Self {
        let thread_count = num_cpus::get();
        let pool = Arc::new(
            threadpool::Builder::new()
                .num_threads(thread_count)
                .thread_stack_size(2_000_000)
                .thread_name("Executor".to_owned())
                .build(),
        );
        let lock_pair = Arc::new((Mutex::new(false), Condvar::new()));
        let result = Self {
            cancel: Arc::new(AtomicBool::new(false)),
            lock_pair,
            pool,
            queue: TaskQueue::new(),
            started: Arc::new(AtomicBool::new(false)),
        };
        result.start();
        result
    }
}
impl Executor {
    fn load(&self) -> bool {
        self.started.load(std::sync::atomic::Ordering::Acquire)
    }

    fn store(&self, val: bool) {
        self.started
            .store(val, std::sync::atomic::Ordering::Release);
    }
}

impl Executor {
    pub fn spawn<Fut>(&mut self, task: Fut) -> Task
    where
        Fut: Future<Output = ()> + 'static + Send,
    {
        let task = Task {
            future: Arc::new(Mutex::new(Box::pin(task))),
            cancelled: false,
            complete: Arc::new(AtomicBool::new(false)),
        };
        self.queue.push(task.clone());

        if !self.load() {
            self.notify();
        }
        task
    }

    fn notify(&self) {
        self.store(true);
        let pair2 = self.lock_pair.clone();
        std::thread::spawn(move || {
            let (lock, cvar) = &*pair2;
            let mut started = lock.lock();
            *started = true;
            cvar.notify_one();
        });
    }

    pub fn cancel(&self) {
        self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        *self.lock_pair.0.lock() = false;
        self.store(false);
        while self.queue.clone().pop().is_some() {}
        self.cancel
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn run(&mut self) {
        loop {
            if !self.cancel.load(std::sync::atomic::Ordering::SeqCst) {
                while let Some(task) = self.queue.pop() {
                    let mut queue = self.queue.clone();
                    if !task.completed() {
                        self.pool.execute(move || {
                            let notifier = Arc::new(Notifier::default());
                            let waker = notifier.clone().into_waker();
                            let task_clone = task.clone();
                            let mut task = task;
                            #[allow(unused_mut)]
                            let mut task = unsafe { Pin::new_unchecked(&mut task) };
                            let mut cx = Context::from_waker(&waker);
                            let task_clone = task_clone.clone();
                            match task.borrow_mut().as_mut().poll(&mut cx) {
                                std::task::Poll::Ready(_) => {}
                                std::task::Poll::Pending => {
                                    queue.push(task_clone);
                                }
                            }
                        });
                    }
                }
            } else {
                self.poll_all();
                return;
            }
        }
    }

    fn poll_all(&self) {
        self.pool.join();
    }

    pub fn start(&self) {
        let lock_pair = self.lock_pair.clone();
        let mut executor = self.clone();
        std::thread::spawn(move || {
            let (lock, cvar) = &*lock_pair;
            let mut started = lock.lock();
            while !*started {
                cvar.wait(&mut started);
            }
            executor.run();
        });
    }
}
