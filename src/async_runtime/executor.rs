use crate::{executors::waker_helper, threadpool_impl::{Channel, ThreadPool}};

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
        let queue: Channel<Task> = self.task_queue.clone();
        let cancelled: Arc<AtomicBool> = self.cancelled.clone();
        let pool: Arc<ThreadPool> = self.pool.clone();
        std::thread::spawn(move || loop {
            let Some(task) = queue.clone().dequeue() else {
                return;
            };
            if cancelled.load(Ordering::Acquire) || task.is_cancelled() {
                continue;
            }
            let queue: Channel<Task> = queue.clone();
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
        self.cancelled.store(true, Ordering::Release);
        self.poll_all();
        self.pool.clear();
        self.cancelled.store(false, Ordering::Release);
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

fn block_task_with(task: Task, queue: &Channel<Task>) {
    if task.is_completed() || task.is_cancelled() {
        return;
    }
    let task_clone = task.clone();
    let waker: Waker = waker_helper(|| {});
    let mut context: Context<'_> = Context::from_waker(&waker);
    let Ok(mut future) = task.lock() else {
        return;
    };
    match future.as_mut().poll(&mut context) {
        Poll::Ready(()) => task_clone.complete(),
        Poll::Pending => {
            if !task_clone.is_cancelled() {
                queue.enqueue(task_clone);
            }
        }
    }
}
