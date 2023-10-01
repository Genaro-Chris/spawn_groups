use crate::async_runtime::{executor::Executor, task::Task};
use crate::async_stream::stream::AsyncStream;
use crate::executors::{block_on, block_task};
use crate::shared::{initializible::Initializible, priority::Priority};
use async_mutex::Mutex;
use num_cpus;
use std::{
    future::Future,
    sync::{atomic::AtomicBool, Arc},
};
use threadpool::ThreadPool;

type Lock = Arc<Mutex<Vec<(Priority, Task)>>>;

pub struct RuntimeEngine<ItemType> {
    pub(crate) iter: Lock,
    pub(crate) engine: ThreadPool,
    pub(crate) runtime: Executor,
    pub(crate) stream: AsyncStream<ItemType>,
    count: Box<usize>,
    wait_flag: Arc<AtomicBool>,
}

impl<ItemType> Initializible for RuntimeEngine<ItemType> {
    fn init() -> Self {
        let thread_count = num_cpus::get();
        let engine = threadpool::Builder::new()
            .num_threads(thread_count)
            .thread_name("RuntimeEngine".to_owned())
            .build();
        let runtime = Executor::default();
        Self {
            engine,
            iter: Arc::new(Mutex::new(vec![])),
            stream: AsyncStream::new(),
            runtime,
            count: Box::new(0),
            wait_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub fn cancel(&mut self) {
        let lock = self.iter.clone();
        self.store(true);
        self.runtime.cancel();
        self.engine.execute(move || {
            block_on(async move {
                let mut iter = lock.lock().await;
                while iter.pop().is_some() {}
            });
        });
        self.stream.cancel_tasks();
        self.poll();
    }
}

impl<ValueType: Send + 'static> RuntimeEngine<ValueType> {
    pub fn wait_for_all_tasks_non_async(&mut self) {
        *self.count = 0;
        let lock = self.iter.clone();
        self.poll();
        self.runtime.cancel();
        let engine = self.engine.clone();
        let store = self.clone();
        self.engine.execute(|| {
            block_on(async move {
                let mut iter = lock.lock().await;
                iter.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
                store.store(true);
                while let Some((_, handle)) = iter.pop() {
                    engine.execute(move || {
                        block_task(handle);
                    });
                }
            });
        });
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub fn load(&self) -> bool {
        self.wait_flag.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn store(&self, val: bool) {
        self.wait_flag
            .store(val, std::sync::atomic::Ordering::Release);
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub fn write_task<F>(&mut self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + Send + 'static,
    {
        if self.load() {
            self.runtime.start();
            self.store(false);
        }
        *self.count += 1;
        let mut stream_clone = self.stream.clone();
        let mut stream = self.stream.clone();
        let task = self.runtime.spawn(async move {
            stream.insert_item(task.await).await;
            stream.decrement_task_count().await;
        });
        let lock = self.iter.clone();
        self.engine.execute(move || {
            block_on(async move {
                stream_clone.increment().await;
                let mut iter = lock.lock().await;
                iter.push((priority, task));
            });
        });
    }
}

impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub async fn wait_for_all_tasks(&mut self) {
        *self.count = 0;
        let lock = self.iter.clone();
        self.poll();
        self.runtime.cancel();
        let mut iter = lock.lock().await;
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
        let thread_count = num_cpus::get();
        Self {
            iter: self.iter.clone(),
            engine: threadpool::Builder::new()
                .num_threads(thread_count)
                .thread_name("RuntimeEngine".to_owned())
                .build(),
            stream: self.stream.clone(),
            runtime: self.runtime.clone(),
            count: self.count.clone(),
            wait_flag: self.wait_flag.clone(),
        }
    }
}
