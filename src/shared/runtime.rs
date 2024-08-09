use crate::{
    async_runtime::executor::Executor, async_stream::AsyncStream, executors::block_task,
    shared::priority::Priority,
};
use std::{
    collections::BinaryHeap,
    future::Future,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use super::priority_task::PrioritizedTask;

type TaskQueue = Mutex<BinaryHeap<PrioritizedTask>>;

pub(crate) struct RuntimeEngine<ItemType: 'static> {
    prioritized_tasks: TaskQueue,
    runtime: Executor,
    task_count: Arc<AtomicUsize>,
    stream: AsyncStream<ItemType>,
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            prioritized_tasks: Mutex::new(BinaryHeap::with_capacity(1000)),
            runtime: Executor::new(count),
            task_count: Arc::new(AtomicUsize::default()),
            stream: AsyncStream::new(),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn cancel(&mut self) {
        self.runtime.cancel();
        let Ok(mut prioritized_tasks) = self.prioritized_tasks.lock() else {
            return;
        };
        prioritized_tasks.clear();
        self.task_count.store(0, Ordering::Release);
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn stream(&self) -> AsyncStream<ItemType> {
        self.stream.clone()
    }

    pub(crate) fn end(&mut self) {
        self.runtime.cancel();
        let Ok(mut prioritized_tasks) = self.prioritized_tasks.lock() else {
            return;
        };
        prioritized_tasks.clear();
        self.task_count.store(0, Ordering::Release);
        self.runtime.end()
    }
}

impl<ValueType> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks(&self) {
        let Ok(mut prioritized_tasks) = self.prioritized_tasks.lock() else {
            return;
        };
        if prioritized_tasks.is_empty() {
            return;
        }
        self.runtime.cancel();
        prioritized_tasks.retain(|prioritized_task| {
            prioritized_task.cancel_task();
            !prioritized_task.is_completed()
        });
        if prioritized_tasks.is_empty() {
            return;
        }
        while let Some(prioritized_task) = prioritized_tasks.pop() {
            if prioritized_task.is_completed() {
                continue;
            }
            self.runtime.submit(move || block_task(prioritized_task));
        }
        self.task_count.store(0, Ordering::Release);
        drop(prioritized_tasks);
        self.poll()
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn write_task<F>(&self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + 'static,
    {
        let Ok(mut prioritized_tasks) = self.prioritized_tasks.lock() else {
            return;
        };
        self.stream.increment();
        self.task_count.fetch_add(1, Ordering::SeqCst);
        let (stream, task_counter) = (self.stream(), self.task_count.clone());
        prioritized_tasks.push(PrioritizedTask::new(
            priority,
            self.runtime.spawn(async move {
                let task_result = task.await;
                stream.insert_item(task_result).await;
                task_counter.fetch_sub(1, Ordering::SeqCst);
            }),
        ));
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn poll(&self) {
        self.runtime.poll_all();
    }

    pub(crate) fn task_count(&self) -> usize {
        self.task_count.load(Ordering::Acquire)
    }
}
