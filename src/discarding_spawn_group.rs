use crate::shared::{
    initializible::Initializible, priority::Priority, runtime::RuntimeEngine, sharedfuncs::Shared,
};

use std::future::Future;

/// Discarding Spawn Group
///
/// A kind of a spawn group that spawns asynchronous tasks that returns nothing,
/// implicitly waits for all spawned tasks to finish before being dropped
/// and releases all the resources before being dropped unless by
/// explicitly calling ``dont_wait_at_drop()``
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
    runtime: RuntimeEngine<()>,
    wait_at_drop: bool,
}

impl DiscardingSpawnGroup {
    /// Don't implicity wait for spawned child tasks to finish before being dropped
    pub fn dont_wait_at_drop(&mut self) {
        self.wait_at_drop = false;
    }
}

impl DiscardingSpawnGroup {
    /// Instantiates `DiscardingSpawnGroup` with a specific number of threads to use in the underlying threadpool when polling futures
    /// 
    /// # Parameters
    ///
    /// * `num_of_threads`: number of threads to use
    pub fn new(num_of_threads: usize) -> Self {
        Self {
            is_cancelled: false,
            runtime: RuntimeEngine::new(num_of_threads),
            wait_at_drop: false,
        }
    }
}

impl DiscardingSpawnGroup {
    /// Spawns a new task into the spawn group
    ///
    /// # Parameters
    ///
    /// * `priority`: priority to use
    /// * `closure`: an async closure that doesn't return anything
    pub fn spawn_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = <DiscardingSpawnGroup as Shared>::Result> + Send + 'static,
    {
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
        self.cancel_all_tasks();
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
        if self.runtime.stream().task_count() == 0 {
            return true;
        }
        false
    }
}

impl Drop for DiscardingSpawnGroup {
    fn drop(&mut self) {
        if self.wait_at_drop {
            self.runtime.wait_for_all_tasks();
        } else {
            self.runtime.end()
        }
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
            runtime: RuntimeEngine::init(),
            wait_at_drop: true,
        }
    }
}
