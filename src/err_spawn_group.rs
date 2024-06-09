use crate::shared::{
    priority::Priority, runtime::RuntimeEngine, sharedfuncs::Shared,
    wait::Waitable,
};
use async_trait::async_trait;
use futures_lite::{Stream, StreamExt};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

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
pub struct ErrSpawnGroup<ValueType: Send + 'static, ErrorType: Send + 'static> {
    /// A field that indicates if the spawn group had been cancelled
    pub is_cancelled: bool,
    count: Arc<AtomicUsize>,
    runtime: RuntimeEngine<Result<ValueType, ErrorType>>,
    wait_at_drop: bool,
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Instantiates `ErrSpawnGroup` with a specific number of threads to use in the underlying threadpool when polling futures
    /// 
    /// # Parameters
    ///
    /// * `num_of_threads`: number of threads to use
    pub fn new(num_of_threads: usize) -> Self {
        Self {
            is_cancelled: false,
            count: Arc::new(AtomicUsize::new(0)),
            runtime: RuntimeEngine::new(num_of_threads),
            wait_at_drop: false,
        }
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
        self.runtime.stream().first().await
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Returns an instance of the `Stream` trait.
    pub fn stream(&self) -> impl Stream<Item = Result<ValueType, ErrorType>> {
        self.runtime.stream()
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    /// Waits for all remaining child tasks for finish.
    pub async fn wait_for_all(&mut self) {
        self.wait().await;
    }
}

impl<ValueType: Send, ErrorType: Send> ErrSpawnGroup<ValueType, ErrorType> {
    fn increment_count(&self) {
        self.count.fetch_add(1, Ordering::Acquire);
    }

    fn count(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    fn decrement_count_to_zero(&self) {
        self.count.store(0, Ordering::Release);
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
        if self.count() == 0 || self.runtime.stream().task_count() == 0 {
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
        let buffer_count: usize = self.runtime.stream().buffer_count().await;
        if buffer_count == of_count {
            let mut count: usize = of_count;
            let mut results: Vec<Result<ValueType, ErrorType>> = vec![];
            while count != 0 {
                if let Some(result) = self.runtime.stream().next().await {
                    results.push(result);
                    count -= 1;
                }
            }
            return results;
        }
        if of_count > self.count() {
            panic!("The argument supplied cannot be greater than the number of spawned child tasks")
        }
        let mut count: usize = of_count;
        let mut results: Vec<Result<ValueType, ErrorType>> = vec![];
        while count != 0 {
            if let Some(result) = self.runtime.stream().next().await {
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
            self.runtime.wait_for_all_tasks();
        } else {
            self.runtime.end()
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
        self.increment_count();
        self.runtime.write_task(priority, closure);
    }

    fn cancel_all_tasks(&mut self) {
        self.runtime.cancel();
        self.is_cancelled = true;
        self.decrement_count_to_zero();
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

impl<ValueType: Send, ErrorType: Send> Stream for ErrSpawnGroup<ValueType, ErrorType> {
    type Item = Result<ValueType, ErrorType>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.runtime.stream().poll_next(cx)
    }
}

#[async_trait]
impl<ValueType: Send + 'static, ErrorType: Send + 'static> Waitable
    for ErrSpawnGroup<ValueType, ErrorType>
{
    async fn wait(&self) {
        self.runtime.wait_for_all_tasks();
        self.decrement_count_to_zero();
    }
}
