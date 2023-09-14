use crate::async_stream::stream::{AsyncStream, AsyncIterator};
use crate::shared::{
    initializible::Initializible,
    priority::Priority, runtime::RuntimeEngine, sharedfuncs::Shared, wait::Waitable,
};
use async_std::stream::Stream;
use async_trait::async_trait;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

/// Spawn Group
///
/// A kind of a spawn group that spawns asynchronous child tasks that returns a value of ValueType,
/// that implicitly wait for the spawned tasks to return before being dropped
///
/// Child tasks are spawned by calling either ``spawn_task()`` or ``spawn_task_unless_cancelled()`` methods.
///
/// Running child tasks can be cancelled by calling ``cancel_all()`` method.
///
/// Child tasks spawned to a spawn group execute concurrently, and may be scheduled in
/// any order.
///
/// It dereferences into a ``futures`` crate ``Stream`` type where the results of each child task is stored and it pops out the result in First-In First-Out
/// FIFO order whenever it is being used
///
pub struct SpawnGroup<ValueType: Send + 'static> {
    /// A field that indicates if the spawn group had been cancelled
    pub is_cancelled: bool,
    count: Box<usize>,
    runtime: RuntimeEngine<ValueType>,
}

impl<ValueType: Send> SpawnGroup<ValueType> {
    pub(crate) fn new() -> Self {
        Self::init()
    }
}

impl<ValueType: Send + 'static> SpawnGroup<ValueType> {
    /// Spawns a new task into the spawn group
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that return a value of type ``ValueType``
    pub fn spawn_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <SpawnGroup<ValueType> as Shared>::Result> + Send + 'static,
    {
        self.add_task(priority, closure);
    }

    /// Spawn a new task only if the group is not cancelled yet,
    /// otherwise does nothing
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that return a value of type ``ValueType``
    pub fn spawn_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <SpawnGroup<ValueType> as Shared>::Result> + Send + 'static,
    {
        self.add_task_unlessed_cancelled(priority, closure);
    }

    /// Cancels all running task in the spawn group
    pub fn cancel_all(&mut self) {
        self.cancel_all_tasks();
    }
}

impl<ValueType: Send> SpawnGroup<ValueType> {
    /// Returns the first element of the stream, or None if it is empty.
    pub async fn first(&self) -> Option<ValueType> {
        self.runtime.stream.first().await
    }
}

impl<ValueType: Send> SpawnGroup<ValueType> {
    /// Waits for all remaining child tasks for finish.
    pub async fn wait_for_all(&mut self) {
        self.wait().await;
    }
}

impl<ValueType: Send> SpawnGroup<ValueType> {
    /// Waits for a specific number of spawned child tasks to finish and returns their respectively results as a vector  
    ///
    /// # Panic
    /// If the `of_count` parameter is larger than the number of already spawned child tasks, this method panics
    ///
    /// # Parameter
    /// * `of_count`: The number of running child tasks to wait for their results to return
    ///
    /// # Returns
    /// Returns a vector of length `of_count` elements from the spawn group instance
    pub async fn get_chunks(&self, of_count: usize) -> Vec<ValueType> {
        if of_count == 0 {
            return vec![];
        }
        if of_count > *self.count {
            panic!("The argument supplied cannot be greater than the number of spawned child tasks")
        }
        let mut count = of_count;
        let mut stream = self.runtime.stream.clone();
        let buffer_count = stream.buffer_count().await;
        let mut results = vec![];
        if count > buffer_count {
            let wait_count = count - buffer_count;
            self.runtime.wait_for(wait_count)
        }
        while count != 0 {
            if let Some(result) = stream.pop_first().await {
                results.push(result);
                count -= 1;
            }
        }
        results
    }
}

impl<ValueType: Send> SpawnGroup<ValueType> {
    /// A Boolean value that indicates whether the group has any remaining tasks.
    ///
    /// At the start of the body of a ``with_spawn_group()`` call, , or before calling ``spawn_task`` or ``spawn_task_unless_cancelled`` methods
    /// the spawn group is always empty.
    ///  
    /// # Returns
    /// - true: if there's no child task still running
    /// - false: if any child task is still running
    pub fn is_empty(&self) -> bool {
        if *self.count == 0 || self.runtime.stream.clone().task_count() == 0 {
            return true;
        }
        false
    }
}

impl<ValueType: Send> Clone for SpawnGroup<ValueType> {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            is_cancelled: self.is_cancelled,
            count: self.count.clone(),
        }
    }
}

impl<ValueType: Send + 'static> Deref for SpawnGroup<ValueType> {
    type Target = AsyncIterator<ValueType>;
    fn deref(&self) -> &Self::Target {
        self.runtime.wait_for(*self.count);
        self
    }
}

impl<ValueType: Send + 'static> DerefMut for SpawnGroup<ValueType> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.runtime.wait_for(*self.count);
        self
    }
}

impl<ValueType: Send> Drop for SpawnGroup<ValueType> {
    fn drop(&mut self) {
        self.runtime.wait_for(*self.count);
    }
}

impl<ValueType: Send> Initializible for SpawnGroup<ValueType> {
    fn init() -> Self {
        SpawnGroup {
            runtime: RuntimeEngine::init(),
            is_cancelled: false,
            count: Box::new(0),
        }
    }
}

impl<ValueType: Send + 'static> Shared for SpawnGroup<ValueType> {
    type Result = ValueType;

    fn add_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static,
    {
        *self.count += 1;
        let task = async_std::task::spawn(closure);
        self.runtime.write_task(priority, task);
    }

    fn cancel_all_tasks(&mut self) {
        self.runtime.cancel();
        self.is_cancelled = true;
        *self.count = 0;
    }

    fn add_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static,
    {
        if !self.is_cancelled {
            self.add_task(priority, closure)
        }
    }
}

impl<ValueType: Send> Stream for SpawnGroup<ValueType> {
    type Item = ValueType;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let pinned_stream = Pin::new(&mut self.runtime.stream);
        <AsyncStream<Self::Item> as Stream>::poll_next(pinned_stream, cx)
    }
}

unsafe impl<ValueType: Send> Sync for SpawnGroup<ValueType> {}

unsafe impl<ValueType: Send> Send for SpawnGroup<ValueType> {}

#[async_trait]
impl<ValueType: Send + 'static> Waitable for SpawnGroup<ValueType> {
    async fn wait(&mut self) {
        self.runtime.wait_for_all_tasks().await;
        *self.count = 0;
    }
}
