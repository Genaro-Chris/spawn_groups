use crate::async_runtime::{executor::Executor, task::Task};
use crate::async_stream::AsyncStream;
use crate::executors::block_task;
use crate::shared::{initializible::Initializible, priority::Priority};
use parking_lot::{Mutex, MutexGuard};
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use threadpool::{Builder, ThreadPool};

type Lock = Arc<Mutex<Vec<(Priority, Task)>>>;

pub struct RuntimeEngine<ItemType> {
    iter: Lock,
    engine: ThreadPool,
    runtime: Executor,
    stream: AsyncStream<ItemType>,
    wait_flag: Arc<AtomicBool>,
}

impl<ItemType> Initializible for RuntimeEngine<ItemType> {
    fn init() -> Self {
        Self {
            engine: Builder::new()
                .thread_name("RuntimeEngine".to_owned())
                .build(),
            iter: Arc::new(Mutex::new(vec![])),
            stream: AsyncStream::new(),
            runtime: Executor::new(),
            wait_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn cancel(&mut self) {
        let lock: Arc<Mutex<Vec<(Priority, Task)>>> = self.iter.clone();
        self.store(true);
        self.runtime.cancel();
        self.engine.execute(move || {
            let mut iter: MutexGuard<'_, Vec<(Priority, Task)>> = lock.lock();
            while iter.pop().is_some() {}
        });
        self.stream.cancel_tasks();
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn stream(&self) -> AsyncStream<ItemType> {
        self.stream.clone()
    }
}

impl<ValueType: Send + 'static> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks_non_async(&self) {
        let lock: Arc<Mutex<Vec<(Priority, Task)>>> = self.iter.clone();
        self.poll();
        self.runtime.cancel();
        let engine: ThreadPool = self.engine.clone();
        let cloned: RuntimeEngine<ValueType> = self.clone();
        self.engine.execute(move || {
            let mut iter: MutexGuard<'_, Vec<(Priority, Task)>> = lock.lock();
            iter.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
            cloned.store(true);
            while let Some((_, handle)) = iter.pop() {
                engine.execute(move || {
                    block_task(handle);
                });
            }
        });
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn load(&self) -> bool {
        self.wait_flag.load(Ordering::Acquire)
    }

    pub(crate) fn store(&self, val: bool) {
        self.wait_flag.store(val, Ordering::Release);
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub(crate) fn write_task<F>(&mut self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + Send + 'static,
    {
        if self.load() {
            self.runtime.start();
            self.store(false);
        }
        self.stream.increment();
        let mut stream: AsyncStream<ItemType> = self.stream.clone();
        let task = self.runtime.spawn(async move {
            stream.insert_item(task.await).await;
            stream.decrement_task_count();
        });
        let lock: Arc<Mutex<Vec<(Priority, Task)>>> = self.iter.clone();
        self.engine.execute(move || {
            let mut iter: MutexGuard<'_, Vec<(Priority, Task)>> = lock.lock();
            iter.push((priority, task));
        });
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub(crate) async fn wait_for_all_tasks(&self) {
        let lock: Arc<Mutex<Vec<(Priority, Task)>>> = self.iter.clone();
        self.poll();
        self.runtime.cancel();
        let mut iter: MutexGuard<'_, Vec<(Priority, Task)>> = lock.lock();
        iter.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        self.store(true);
        while let Some((_, handle)) = iter.pop() {
            self.engine.execute(move || {
                block_task(handle);
            });
        }
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn poll(&self) {
        self.engine.join();
    }
}

impl<ItemType> Clone for RuntimeEngine<ItemType> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            engine: self.engine.clone(),
            stream: self.stream.clone(),
            runtime: self.runtime.clone(),
            wait_flag: self.wait_flag.clone(),
        }
    }
}
