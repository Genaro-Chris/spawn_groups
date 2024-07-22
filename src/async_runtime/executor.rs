use crate::{
    executors::{IntoWaker, Notifier},
    threadpool_impl::{Channel, ThreadPool},
};

use super::task::Task;

use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

pub struct Executor {
    pool: Arc<ThreadPool>,
    cancelled: Arc<AtomicBool>,
    task_queue: Channel<Task>,
}

impl Executor {
    pub(crate) fn new(count: usize) -> Self {
        let result: Executor = Self {
            pool: Arc::new(ThreadPool::new(count)),
            cancelled: Arc::new(AtomicBool::new(false)),
            task_queue: Channel::new(),
        };
        result.start();
        result
    }
}

impl Executor {
    fn start(&self) {
        let queue = self.task_queue.clone();
        let cancelled = self.cancelled.clone();
        let pool = self.pool.clone();
        std::thread::spawn(move || loop {
            let Some(task) = queue.clone().dequeue() else {
                return;
            };
            if cancelled.load(Ordering::Relaxed) || task.is_cancelled() {
                continue;
            }
            let queue = queue.clone();
            pool.submit(move || block_task_with(task, &queue))
        });
    }

    pub(crate) fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + Send + 'static,
    {
        self.pool.submit(task);
    }

    pub(crate) fn spawn<F>(&self, task: F) -> Task
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task: Task = Task::new(task);
        self.task_queue.enqueue(task.clone());
        task
    }

    pub(crate) fn cancel(&self) {
        self.pool.clear();
        self.cancelled.store(true, Ordering::Relaxed);
        self.poll_all();
        self.pool.clear();
        self.cancelled.store(false, Ordering::Relaxed);
    }

    pub(crate) fn poll_all(&self) {
        self.pool.wait_for_all();
    }

    pub(crate) fn end(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.task_queue.close();
        self.pool.end();
    }
}

thread_local! {
    static TASK_WAKER: Waker = {
        Arc::new(Notifier::default()).into_waker()
    };
}

fn block_task_with(future: Task, queue: &Channel<Task>) {
    if future.is_completed() || future.is_cancelled() {
        return;
    }
    let task = future.clone();
    let waker_result = TASK_WAKER.try_with(|waker| waker.clone());
    match waker_result {
        Ok(waker) => {
            let mut context: Context<'_> = Context::from_waker(&waker);
            let Ok(mut future) = future.lock() else {
                return;
            };
            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => task.complete(),
                Poll::Pending => {
                    if !task.is_cancelled() {
                        queue.enqueue(task.clone());
                    }
                }
            }
        }
        Err(_) => {
            let waker: Waker = Arc::new(Notifier::default()).into_waker();
            let mut context: Context<'_> = Context::from_waker(&waker);
            let Ok(mut future) = future.lock() else {
                return;
            };
            match future.as_mut().poll(&mut context) {
                Poll::Ready(()) => task.complete(),
                Poll::Pending => {
                    if !task.is_cancelled() {
                        queue.enqueue(task.clone());
                    }
                }
            }
        }
    }
}
