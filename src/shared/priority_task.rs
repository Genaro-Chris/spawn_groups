use std::{
    cmp::Ordering,
    future::Future,
    sync::{Arc, Barrier},
};

use crate::threadpool_impl::TaskPriority;

use super::{task::Task, task_enum::TaskOrBarrier};

pub(crate) struct PrioritizedTask<T> {
    pub(crate) task: TaskOrBarrier<T>,
    priority: TaskPriority,
}

impl<T> PartialEq for PrioritizedTask<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl<T> Eq for PrioritizedTask<T> {}

impl<T> PrioritizedTask<T> {
    pub(crate) fn new<F: Future<Output = T> + 'static>(priority: TaskPriority, future: F) -> Self {
        Self {
            task: TaskOrBarrier::Task(Task::new(future)),
            priority,
        }
    }

    pub(crate) fn new_with(priority: TaskPriority, barrier: Arc<Barrier>) -> Self {
        Self {
            task: TaskOrBarrier::Barrier(barrier),
            priority,
        }
    }
}

impl<T> Ord for PrioritizedTask<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl<T> PartialOrd for PrioritizedTask<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(other.cmp(self))
    }
}
