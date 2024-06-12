use crate::{pin_future, threadpool_impl::ThreadPool};

use super::{notifier::Notifier, task::Task};

use cooked_waker::IntoWaker;

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
}

impl Executor {
    pub(crate) fn new(count: usize) -> Self {
        let result: Executor = Self {
            pool: Arc::new(ThreadPool::new(count)),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        result
    }
}

impl Executor {
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
        self.spawn_task(task.clone());
        task
    }

    fn spawn_task(&self, task: Task) {
        let executor = self.clone();
        if self.cancelled.load(Ordering::Acquire) {
            return;
        }
        self.submit(move || {
            let waker: Waker = Arc::new(Notifier::default()).into_waker();
            pin_future!(task);
            let mut cx: Context<'_> = Context::from_waker(&waker);
            match task.as_mut().poll(&mut cx) {
                Poll::Ready(()) => task.complete(),
                Poll::Pending => {
                    cx.waker().wake_by_ref();
                    executor.spawn_task(task.clone());
                }
            }
        });
    }

    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            cancelled: self.cancelled.clone(),
        }
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

    pub(crate) fn end(&mut self) {}
}
