use crate::{async_stream::AsyncStream, shared::priority::Priority, threadpool_impl::ThreadPool};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use super::priority_task::PrioritizedTask;

pub(crate) struct RuntimeEngine<ItemType> {
    stream: AsyncStream<ItemType>,
    pool: ThreadPool,
    task_count: Arc<AtomicUsize>,
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            pool: ThreadPool::new(count),
            stream: AsyncStream::new(),
            task_count: Arc::new(AtomicUsize::default()),
        }
    }
}

impl<ItemType> Default for RuntimeEngine<ItemType> {
    fn default() -> Self {
        Self {
            pool: ThreadPool::default(),
            stream: AsyncStream::new(),
            task_count: Arc::new(AtomicUsize::default()),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn cancel(&self) {
        self.pool.clear();
        self.pool.wait_for_all();
        self.task_count.store(0, Ordering::Relaxed);
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn stream(&self) -> AsyncStream<ItemType> {
        self.stream.clone()
    }

    pub(crate) fn end(&self) {
        self.pool.clear();
        self.pool.wait_for_all();
        self.task_count.store(0, Ordering::Relaxed);
        self.pool.end()
    }
}

impl<ValueType> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks(&self) {
        self.poll();
        self.task_count.store(0, Ordering::Relaxed);
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn write_task(&mut self, priority: Priority, task: impl Future<Output = ItemType>) {
        let (stream, task_counter) = (self.stream(), self.task_count.clone());
        stream.increment();
        task_counter.fetch_add(1, Ordering::Relaxed);
        self.pool
            .submit(PrioritizedTask::new(priority.into(), async move {
                let task_result = task.await;
                stream.insert_item(task_result).await;
                task_counter.fetch_sub(1, Ordering::Relaxed);
            }));
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    fn poll(&self) {
        self.pool.wait_for_all();
    }

    pub(crate) fn task_count(&self) -> usize {
        self.task_count.load(Ordering::Acquire)
    }
}
