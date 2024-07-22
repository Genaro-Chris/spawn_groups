use crate::{
    async_runtime::{executor::Executor, task::Task},
    async_stream::AsyncStream,
    executors::{block_on, block_task},
    shared::priority::Priority,
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};

type TaskQueue = Arc<Mutex<Vec<(Priority, Task)>>>;

pub struct RuntimeEngine<ItemType> {
    tasks: TaskQueue,
    runtime: Executor,
    stream: AsyncStream<ItemType>,
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(vec![])),
            stream: AsyncStream::new(),
            runtime: Executor::new(count),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn cancel(&mut self) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        self.runtime.cancel();
        tasks.clear();
        self.stream.cancel_tasks();
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn stream(&self) -> AsyncStream<ItemType> {
        self.stream.clone()
    }

    pub(crate) fn end(&mut self) {
        self.cancel();
        self.runtime.end()
    }
}

impl<ValueType: Send + 'static> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks(&self) {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        if tasks.is_empty() {
            return;
        }
        self.runtime.cancel();
        tasks.retain(|(_, task)| {
            task.cancel();
            !task.is_completed()
        });
        tasks.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        if tasks.is_empty() {
            return;
        }
        while let Some((_, task)) = tasks.pop() {
            if task.is_completed() {
                continue;
            }
            self.runtime.submit(move || block_task(task));
        }
        self.poll()
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub(crate) fn write_task<F>(&self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + Send + 'static,
    {
        let Ok(mut tasks) = self.tasks.lock() else {
            return;
        };
        self.stream.increment();
        let stream: AsyncStream<ItemType> = self.stream();
        tasks.push((
            priority,
            self.runtime.spawn(async move {
                let task_result = task.await;
                block_on(async { stream.insert_item(task_result).await });
                stream.decrement_task_count();
            }),
        ));
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn poll(&self) {
        self.runtime.poll_all();
    }
}
