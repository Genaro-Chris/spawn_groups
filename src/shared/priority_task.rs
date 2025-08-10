use std::{
    future::Future,
    sync::{Arc, Barrier},
};

use crate::threadpool_impl::TaskPriority;

use super::{task::Task, task_enum::TaskOrBarrier};

pub(crate) struct PrioritizedTask<T> {
    pub(crate) task: TaskOrBarrier<T>,
    priority: TaskPriority,
}

impl<T> PrioritizedTask<T> {
    pub(crate) fn priority(&self) -> TaskPriority {
        self.priority.clone()
    }
}

impl<T> PrioritizedTask<T> {
    pub(crate) fn new(priority: TaskPriority, future: impl Future<Output = T>) -> Self {
        Self {
            task: TaskOrBarrier::Task(Task::new(future)),
            priority,
        }
    }

    pub(crate) fn new_with(barrier: Arc<Barrier>) -> Self {
        Self {
            task: TaskOrBarrier::Barrier(barrier),
            priority: TaskPriority::Wait,
        }
    }
}
