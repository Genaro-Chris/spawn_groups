use std::collections::VecDeque;
pub(super) struct Inner<T> {
    pub(crate) buffer: VecDeque<T>,
    count: usize,
    task_count: usize,
    cancelled: bool,
}

impl<ItemType> Inner<ItemType> {
    pub(super) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            cancelled: false,
            count: 0,
            task_count: 0,
        }
    }

    pub(crate) fn increment_count(&mut self) {
        self.count += 1;
    }

    pub(crate) fn decrement_count(&mut self) {
        if self.count > 0 {
            self.count -= 1;
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.count
    }

    pub(crate) fn task_count(&self) -> usize {
        self.task_count
    }

    pub(crate) fn increment_task_count(&mut self) {
        self.task_count += 1;
    }

    pub(crate) fn decrement_task_count(&mut self) {
        if self.task_count > 0 {
            self.task_count -= 1;
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    pub(crate) fn cancel_tasks(&mut self) {
        self.task_count = 0;
        self.cancelled = true;
    }
}
