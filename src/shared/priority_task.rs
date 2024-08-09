use std::{
    cmp::Ordering,
    ops::{Deref, DerefMut},
};

use crate::{async_runtime::task::Task, Priority};

pub(crate) struct PrioritizedTask {
    task: Task,
    priority: Priority,
}

impl Deref for PrioritizedTask {
    type Target = Task;

    fn deref(&self) -> &Self::Target {
        &self.task
    }
}

impl DerefMut for PrioritizedTask {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.task
    }
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for PrioritizedTask {}

impl PrioritizedTask {
    pub(crate) fn new(priority: Priority, task: Task) -> Self {
        Self { task, priority }
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
