use crate::{
    pin_future,
    threadpool_impl::{Channel, ThreadPool},
};

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

#[derive(Clone)]
pub struct Executor {
    pool: Arc<ThreadPool>,
    queue: Channel<Task>,
    cancelled: Arc<AtomicBool>,
}

impl Executor {
    pub(crate) fn new(count: usize) -> Self {
        let result: Executor = Self {
            pool: Arc::new(ThreadPool::new(count)),
            queue: Channel::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let result_clone: Executor = result.clone();
        std::thread::spawn(move || {
            result_clone.run();
        });
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
        self.queue.enqueue(task.clone());
        task
    }

    pub(crate) fn cancel(&self) {
        self.queue.clear();
        self.cancelled.store(true, Ordering::Relaxed);
        self.poll_all();
        self.queue.clear();
        self.cancelled.store(false, Ordering::Relaxed);
    }

    fn run(&self) {
        while let Some(task) = self.queue.dequeue() {
            if self.cancelled.load(Ordering::Acquire) {
                continue;
            }
            let queue: Channel<Task> = self.queue.clone();
            self.submit(move || {
                let waker: Waker = Arc::new(Notifier::default()).into_waker();
                pin_future!(task);
                let mut cx: Context<'_> = Context::from_waker(&waker);
                match task.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => (),
                    Poll::Pending => {
                        queue.enqueue(task.clone());
                    }
                }
            });
        }
    }

    pub(crate) fn poll_all(&self) {
        self.pool.wait_for_all();
    }

    pub(crate) fn end(&mut self) {
        self.queue.close();
        self.queue.clear();
    }
}
