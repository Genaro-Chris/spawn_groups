use super::task::Task;
use crate::arcimpl::arclock::ARCLock;
use std::{collections::VecDeque, iter::Iterator};

#[derive(Clone, Default)]
pub struct TaskQueue {
    buffer: ARCLock<VecDeque<Task>>,
}

impl TaskQueue {
    pub(crate) fn push(&self, task: &Task) {
        self.buffer
            .update_while_locked(|buffer| buffer.push_back(task.clone()));
    }
}

impl TaskQueue {
    pub(crate) fn drain_all(&self) {
        self.buffer.update_while_locked(|buffer| buffer.clear());
    }
}

impl Iterator for TaskQueue {
    type Item = Task;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.update_while_locked(|buffer| {
            let Some(task) = buffer.pop_front() else {
                return None;
            };
            if !task.is_completed() {
                return Some(task);
            }
            None
        })
    }
}
