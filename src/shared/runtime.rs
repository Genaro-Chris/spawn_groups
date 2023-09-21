use crate::async_runtime::executor::Executor;
use crate::async_runtime::task::Task;
use crate::async_stream::stream::AsyncStream;
use crate::shared::{initializible::Initializible, priority::Priority};
use async_mutex::Mutex;
use num_cpus;
use std::sync::atomic::AtomicBool;
use std::{future::Future, sync::Arc};
use threadpool::ThreadPool;
type Lock = Arc<Mutex<Vec<(Priority, Task)>>>;

pub struct RuntimeEngine<ItemType> {
    pub(crate) iter: Lock,
    pub(crate) engine: ThreadPool,
    pub(crate) runtime: Executor,
    pub stream: AsyncStream<ItemType>,
    count: Box<usize>,
    wait_for: Arc<AtomicBool>,
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
            wait_for: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub fn cancel(&mut self) {
        let lock = self.iter.clone();
        let stream = self.stream.clone();
        self.store(true);
        self.runtime.cancel();
        self.engine.execute(move || {
            futures_lite::future::block_on(async move {
                let mut iter = lock.lock().await;
                while let Some((_, handle)) = iter.pop() {
                    if !handle.completed() {
                        handle.cancel().await;
                    }
                }
            });
        });
        stream.cancel_tasks();
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub fn load(&self) -> bool {
        self.wait_for.load(std::sync::atomic::Ordering::Acquire)
    }

    pub(crate) fn store(&self, val: bool) {
        self.wait_for
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
        let mut stream = self.stream.clone();
        stream.increment();
        let task = self.runtime.spawn(async move {
            stream.insert_item(task.await).await;
            stream.decrement_task_count().await;
        });
        let lock = self.iter.clone();
        self.engine.execute(move || {
            futures_lite::future::block_on(async move {
                let mut iter = lock.lock().await;
                iter.push((priority, task));
            });
        });
    }
}
#[allow(dead_code)]
impl<ItemType: Send + 'static> RuntimeEngine<ItemType> {
    pub async fn wait_for_all_tasks(&mut self) {
        let lock = self.iter.clone();
        self.poll();
        let engine = self.engine.clone();
        let mut iter = lock.lock().await;
        iter.sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        self.runtime.cancel();
        self.store(true);
        while let Some((_, handle)) = iter.pop() {
            engine.execute(move || {
                let handle_clone = handle.clone();
                if handle_clone.completed() || handle_clone.cancelled {
                    return;
                }
                futures_lite::future::block_on(handle);
            });
        }
        *self.count = 0;
        self.poll();
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    fn poll(&self) {
        self.engine.join();
    }
}

impl<ItemType> Drop for RuntimeEngine<ItemType> {
    fn drop(&mut self) {
        self.poll();
    }
}

impl<ItemType> Clone for RuntimeEngine<ItemType> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            engine: threadpool::Builder::new()
                .num_threads(num_cpus::get())
                .thread_name("RuntimeEngine".to_owned())
                .build(),
            stream: self.stream.clone(),
            runtime: self.runtime.clone(),
            count: self.count.clone(),
            wait_for: self.wait_for.clone(),
        }
    }
}
