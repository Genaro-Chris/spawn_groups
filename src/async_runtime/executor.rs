use crate::{
    executors::{pair, Suspender, WAKER_PAIR},
    threadpool_impl::ThreadPool,
};

use super::task::Task;

use std::{
    future::Future,
    task::{Context, Poll},
};

pub struct Executor {
    pool: ThreadPool,
}

impl Executor {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            pool: ThreadPool::new(count),
        }
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
        F: Future<Output = ()> + 'static,
    {
        let task: Task = Task::new(task);
        self.async_poll_task(task.clone());
        task
    }

    fn async_poll_task(&self, task: Task) {
        if task.is_completed() || task.is_cancelled() {
            return;
        }

        self.submit(move || {
            WAKER_PAIR.with(move |waker_pair| {
                match waker_pair.try_borrow_mut() {
                    Ok(waker_pair) => {
                        let (suspender, waker) = &*waker_pair;
                        let mut context: Context<'_> = Context::from_waker(waker);
                        poll_task(task, suspender, &mut context)
                    }
                    Err(_) => {
                        let (suspender, waker) = pair();
                        let mut context: Context<'_> = Context::from_waker(&waker);
                        poll_task(task, &suspender, &mut context)
                    }
                };
            });
        });
    }

    pub(crate) fn cancel(&self) {
        self.pool.clear();
        self.poll_all();
        self.pool.clear();
    }

    pub(crate) fn poll_all(&self) {
        self.pool.wait_for_all();
    }

    pub(crate) fn end(&self) {
        self.pool.end();
    }
}

#[inline]
fn poll_task(task: Task, suspender: &Suspender, context: &mut Context<'_>) {
    let mut task = task;
    loop {
        match task.poll_task(context) {
            Poll::Ready(()) => {
                return;
            }
            Poll::Pending => {
                suspender.suspend();
                if task.is_cancelled() {
                    return;
                }
            }
        }
    }
}
