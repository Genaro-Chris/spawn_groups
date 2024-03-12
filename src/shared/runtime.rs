use crate::{
    async_runtime::{executor::Executor, task::Task},
    async_stream::AsyncStream,
    executors::block_task,
    shared::{initializible::Initializible, priority::Priority},
};
use parking_lot::Mutex;
use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

type TaskQueue = Arc<Mutex<Vec<(Priority, Task)>>>;

pub struct RuntimeEngine<ItemType> {
    tasks: TaskQueue,
    runtime: Executor,
    stream: AsyncStream<ItemType>,
    wait_flag: Arc<AtomicBool>,
}

impl<ItemType> Initializible for RuntimeEngine<ItemType> {
    fn init() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(vec![])),
            stream: AsyncStream::new(),
            runtime: Executor::default(),
            wait_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(vec![])),
            stream: AsyncStream::new(),
            runtime: Executor::new(count),
            wait_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn cancel(&mut self) {
        self.store(true);
        self.runtime.cancel();
        self.tasks.lock().clear();
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
        self.tasks.lock().clear();
    }
}

impl<ValueType: Send + 'static> RuntimeEngine<ValueType> {
    pub(crate) fn wait_for_all_tasks(&self) {
        self.poll();
        self.runtime.cancel();
        self.tasks.lock().sort_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
        self.store(true);
        while let Some((_, handle)) = self.tasks.lock().pop() {
            self.runtime.submit(move || {
                block_task(handle);
            });
        }
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
    pub(crate) fn write_task<F>(&self, priority: Priority, task: F)
    where
        F: Future<Output = ItemType> + Send + 'static,
    {
        if self.load() {
            self.runtime.start();
            self.store(false);
        }
        self.stream.increment();
        let mut stream: AsyncStream<ItemType> = self.stream();
        let runtime = self.runtime.clone();
        let tasks: Arc<Mutex<Vec<(Priority, Task)>>> = self.tasks.clone();
        self.runtime.submit(move || {
            tasks.lock().push((
                priority,
                runtime.spawn(async move {
                    stream.insert_item(task.await).await;
                    stream.decrement_task_count();
                }),
            ));
        });
    }
}

impl<ItemType> RuntimeEngine<ItemType> {
    pub(crate) fn poll(&self) {
        self.runtime.poll_all();
    }
}
