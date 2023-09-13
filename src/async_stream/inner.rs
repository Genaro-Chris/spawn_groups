use std::collections::VecDeque;
pub(crate) struct Inner<T> {
    pub(crate) buffer: VecDeque<T>,
    pub(crate) count: usize,
    pub(crate) task_count: usize,
    pub(crate) cancelled: bool,
}

impl<ItemType> Inner<ItemType> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::from([]),
            cancelled: false,
            count: 0,
            task_count: 0,
        }
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

    pub(crate) fn cancel_tasks(&mut self) {
        self.task_count = 0;
        self.cancelled = true;
    }
}
