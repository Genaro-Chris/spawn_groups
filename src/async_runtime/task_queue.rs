use super::task::Task;
use parking_lot::Mutex;
use std::{collections::VecDeque, iter::Iterator, sync::Arc};

#[derive(Clone, Default)]
pub struct TaskQueue {
    buffer: Arc<Mutex<VecDeque<Task>>>,
}

impl TaskQueue {
    pub(crate) fn push(&self, task: &Task) {
        self.buffer.lock().push_back(task.clone());
    }
}

impl TaskQueue {
    pub(crate) fn drain_all(&self) {
        self.buffer.lock().clear();
    }
}

impl Iterator for TaskQueue {
    type Item = Task;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(task) = self.buffer.lock().pop_front() else {
            return None;
        };
        if !task.is_completed() {
            return Some(task);
        }
        None
    }
}
