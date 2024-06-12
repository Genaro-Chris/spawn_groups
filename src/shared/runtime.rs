use crate::{
    async_runtime::{executor::Executor, task::Task},
    async_stream::AsyncStream,
    executors::block_task,
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
        self.runtime.cancel();
        self.tasks.lock().unwrap().clear();
        self.stream.cancel_tasks();
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn stream(&self) -> AsyncStream<ItemType> {
        self.stream.clone()
    }

    pub(crate) fn end(&mut self) {
        self.runtime.cancel();
        self.tasks.lock().unwrap().clear();
        self.runtime.end()
    }
}

impl<ValueType: Send + 'static> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks(&self) {
        self.poll();
        self.runtime.cancel();
        if let Ok(mut lock) = self.tasks.lock() {
            lock.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
            while let Some((_, handle)) = lock.pop() {
                self.runtime.submit(move || {
                    block_task(handle);
                });
            }
        }

        self.poll();
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub(crate) fn write_task<F>(&self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + Send + 'static,
    {
        self.stream.increment();
        let mut stream: AsyncStream<ItemType> = self.stream();
        let tasks: Arc<Mutex<Vec<(Priority, Task)>>> = self.tasks.clone();
        tasks.lock().unwrap().push((
            priority,
            self.runtime.spawn(async move {
                stream.insert_item(task.await).await;
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
