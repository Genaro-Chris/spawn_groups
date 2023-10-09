use crate::async_stream::stream::AsyncStream;
use crate::shared::{
    initializible::Initializible, priority::Priority, runtime::RuntimeEngine, sharedfuncs::Shared,
    wait::Waitable,
};
use async_trait::async_trait;
use futures_lite::{Stream, StreamExt};
use std::task::{Context, Poll};
use std::{future::Future, pin::Pin};

/// Err Spawn Group
///
/// A kind of a spawn group that spawns asynchronous child tasks that returns a value of Result<ValueType, ErrorType>,
/// that implicitly wait for the spawned tasks to return before being dropped unless by
/// explicitly calling ``dont_wait_at_drop()``
///
/// Child tasks are spawned by calling either ``spawn_task()`` or ``spawn_task_unless_cancelled()`` methods.
///
/// Running child tasks can be cancelled by calling ``cancel_all()`` method.
///
/// Child tasks spawned to a spawn group execute concurrently, and may be scheduled in
/// any order.
///
/// It dereferences into a ``futures`` crate ``Stream`` type where the results of each finished child task is stored and it pops out the result in First-In First-Out
/// FIFO order whenever it is being used

#[derive(Clone)]
pub struct ErrSpawnGroup<ValueType: Send + 'static, ErrorType: Send + 'static> {
    /// A field that indicates if the spawn group had been cancelled
    pub is_cancelled: bool,
    count: Box<usize>,
    runtime: RuntimeEngine<Result<ValueType, ErrorType>>,
    wait_at_drop: bool,
    polled: bool,
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    pub(crate) fn new() -> Self {
        Self::init()
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Don't implicity wait for spawned child tasks to finish before being dropped
    pub fn dont_wait_at_drop(&mut self) {
        self.wait_at_drop = false;
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Spawns a new task into the spawn group
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that return a value of type ``Result<ValueType, ErrorType>``
    pub fn spawn_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <ErrSpawnGroup<ValueType, ErrorType> as Shared>::Result>
            + Send
            + 'static,
    {
        self.add_task(priority, closure);
    }

    /// Cancels all running task in the spawn group
    pub fn cancel_all(&mut self) {
        self.cancel_all_tasks();
    }

    /// Spawn a new task only if the group is not cancelled yet,
    /// otherwise does nothing
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that return a value of type ``Result<ValueType, ErrorType>``
    pub fn spawn_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <ErrSpawnGroup<ValueType, ErrorType> as Shared>::Result>
            + Send
            + 'static,
    {
        self.add_task_unlessed_cancelled(priority, closure);
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Returns the first element of the stream, or None if it is empty.
    pub async fn first(&self) -> Option<<ErrSpawnGroup<ValueType, ErrorType> as Shared>::Result> {
        self.runtime.stream.first().await
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Waits for all remaining child tasks for finish.
    pub async fn wait_for_all(&mut self) {
        self.wait().await;
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// A Boolean value that indicates whether the group has any remaining tasks.
    ///
    /// At the start of the body of a ``with_err_spawn_group`` function call, or before calling ``spawn_task`` or ``spawn_task_unless_cancelled`` methods
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

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Waits for a specific number of spawned child tasks to finish and returns their respectively result as a vector  
    ///
    /// # Panics
    /// If the `of_count` parameter is larger than the number of already spawned child tasks, this method panics
    ///
    /// Remember whenever you call either ``wait_for_all`` or ``cancel_all`` methods, the child tasks' count reverts back to zero
    ///
    /// # Parameter
    /// * `of_count`: The number of running child tasks to wait for their results to return
    ///
    /// # Returns
    /// Returns a vector of length `of_count` elements from the spawn group instance
    #[deprecated(since = "2.0.0", note = "Buggy")]
    pub async fn get_chunks(&self, of_count: usize) -> Vec<Result<ValueType, ErrorType>> {
        if of_count == 0 {
            return vec![];
        }
        let buffer_count: usize = self.runtime.stream.buffer_count().await;
        if buffer_count == of_count {
            let mut count: usize = of_count;
            let mut results: Vec<Result<ValueType, ErrorType>> = vec![];
            while count != 0 {
                if let Some(result) = self.runtime.stream.clone().next().await {
                    results.push(result);
                    count -= 1;
                }
            }
            return results;
        }
        if of_count > *self.count {
            panic!("The argument supplied cannot be greater than the number of spawned child tasks")
        }
        let mut count: usize = of_count;
        let mut results: Vec<Result<ValueType, ErrorType>> = vec![];
        while count != 0 {
            if let Some(result) = self.runtime.stream.clone().next().await {
                results.push(result);
                count -= 1;
            }
        }
        results
    }
}

impl<ValueType: Send, ErrorType: Send + 'static> Drop for ErrSpawnGroup<ValueType, ErrorType> {
    fn drop(&mut self) {
        if self.wait_at_drop {
            self.runtime.wait_for_all_tasks_non_async();
        }
    }
}

impl<ValueType: Send, ErrorType: Send> Initializible for ErrSpawnGroup<ValueType, ErrorType> {
    fn init() -> Self {
        ErrSpawnGroup::<ValueType, ErrorType> {
            count: Box::new(0),
            is_cancelled: false,
            runtime: RuntimeEngine::init(),
            wait_at_drop: true,
            polled: false,
        }
    }
}

impl<ValueType: Send + 'static, ErrorType: Send + 'static> Shared
    for ErrSpawnGroup<ValueType, ErrorType>
{
    type Result = Result<ValueType, ErrorType>;

    fn add_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static,
    {
        self.polled = false;
        *self.count += 1;
        self.runtime.write_task(priority, closure);
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

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    fn poll_all(&self) {
        self.runtime.poll();
    }
}

impl<ValueType: Send, ErrorType: Send> Stream for ErrSpawnGroup<ValueType, ErrorType> {
    type Item = Result<ValueType, ErrorType>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.polled {
            self.poll_all();
            self.polled = true;
        }
        let pinned_stream: Pin<&mut AsyncStream<Result<ValueType, ErrorType>>> = Pin::new(&mut self.runtime.stream);
        <AsyncStream<Self::Item> as Stream>::poll_next(pinned_stream, cx)
    }
}

#[async_trait]
impl<ValueType: Send + 'static, ErrorType: Send + 'static> Waitable
    for ErrSpawnGroup<ValueType, ErrorType>
{
    async fn wait(&mut self) {
        self.runtime.wait_for_all_tasks().await;
        *self.count = 0;
    }
}
