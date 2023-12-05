use crate::shared::priority::Priority;
use std::future::Future;


/// The basic functionalities between all kinds of spawn groups
pub trait Shared {
    /// A value return when a task is being awaited for
    type Result;
    /// Add a new task into the engine
    fn add_task<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static;
    /// Cancels all running tasks in the engine
    fn cancel_all_tasks(&mut self);
    /// Add a new task only if the engine is not cancelled yet,
    /// otherwise does nothing
    fn add_task_unlessed_cancelled<F>(&mut self, priority: Priority, closure: F)
    where
        F: Future<Output = Self::Result> + Send + 'static;
}
