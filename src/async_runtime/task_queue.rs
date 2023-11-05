use super::task::Task;
use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex};
use std::{collections::VecDeque, iter::Iterator, sync::Arc};

#[derive(Clone, Default)]
pub struct TaskQueue {
    buffer: Arc<Mutex<VecDeque<Task>>>,
}

impl TaskQueue {
    pub(crate) fn push(&self, task: Task) {
        let mut inner_lock: MutexGuard<'_, RawMutex, VecDeque<Task>> = self.buffer.lock();
        inner_lock.push_back(task);
    }
}

impl Iterator for TaskQueue {
    type Item = Task;

    fn next(&mut self) -> Option<Self::Item> {
        let mut inner_lock: MutexGuard<'_, RawMutex, VecDeque<Task>> = self.buffer.lock();
        if inner_lock.is_empty() {
            return None;
        }
        if let Some(task) = inner_lock.pop_front() {
            if !task.is_completed() {
                return Some(task);
            }
        }

        None
    }
}
