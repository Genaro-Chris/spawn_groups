use crate::shared::{
    initializible::Initializible, priority::Priority, runtime::RuntimeEngine, sharedfuncs::Shared,
};

use std::future::Future;

/// Discarding Spawn Group
///
/// A kind of a spawn group that spawns asynchronous tasks that returns nothing,
/// implicitly waits for all spawned tasks to finish before being dropped
/// and releases all the resources before being dropped.
///
/// Child tasks are spawned by calling either ``spawn_task()`` or ``spawn_task_unless_cancelled()`` methods.
///
/// Running child tasks can be cancelled by calling ``cancel_all()`` method.
///
/// Child tasks spawned to a spawn group execute concurrently, and may be scheduled in
/// any order.
///
pub struct DiscardingSpawnGroup {
    /// A field that indicates if the spawn group has been cancelled
    pub is_cancelled: bool,
    pub(crate) count: Box<usize>,
    pub(crate) runtime: RuntimeEngine<()>,
}

impl DiscardingSpawnGroup {
    pub(crate) fn new() -> Self {
        Self::init()
    }
}

impl DiscardingSpawnGroup {
    /// Spawns a new task into the spawn group
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that doesn't return a anything
    pub fn spawn_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <DiscardingSpawnGroup as Shared>::Result> + Send + 'static,
    {
        *self.count += 1;
        self.add_task(priority, closure);
    }

    /// Spawn a new task only if the group is not cancelled yet,
    /// otherwise does nothing
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that return doesn't return anything
    pub fn spawn_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <DiscardingSpawnGroup as Shared>::Result> + Send + 'static,
    {
        self.add_task_unlessed_cancelled(priority, closure);
    }

    /// Cancels all running task in the spawn group
    pub fn cancel_all(&mut self) {
        *self.count = 0;
        self.cancel_all_tasks();
    }
}

impl DiscardingSpawnGroup {
    async fn wait_for_all(&mut self) {
        self.runtime.wait_for_all_tasks().await;
        *self.count = 0;
    }
}

impl DiscardingSpawnGroup {
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

impl Clone for DiscardingSpawnGroup {
    fn clone(&self) -> Self {
        Self {
            is_cancelled: self.is_cancelled,
            count: self.count.clone(),
            runtime: self.runtime.clone(),
        }
    }
}

impl Drop for DiscardingSpawnGroup {
    fn drop(&mut self) {
        futures_lite::future::block_on(async move {
            self.wait_for_all().await;
        });
    }
}

impl Shared for DiscardingSpawnGroup {
    type Result = ();

    fn add_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static,
    {
        self.runtime.write_task(priority, closure);
    }

    fn add_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static,
    {
        if !self.is_cancelled {
            self.add_task(priority, closure)
        }
    }

    fn cancel_all_tasks(&mut self) {
        self.runtime.cancel();
        self.is_cancelled = true;
    }
}

impl Initializible for DiscardingSpawnGroup {
    fn init() -> Self {
        DiscardingSpawnGroup {
            is_cancelled: false,
            count: Box::new(0),
            runtime: RuntimeEngine::init(),
        }
    }
}

unsafe impl Sync for DiscardingSpawnGroup {}

unsafe impl Send for DiscardingSpawnGroup {}
