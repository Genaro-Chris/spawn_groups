use std::collections::VecDeque;

use crate::arcimpl::arclock::ARCLock;

#[derive(Default)]
pub(crate) struct ThreadSafeQueue<ItemType> {
    buffer: ARCLock<VecDeque<ItemType>>,
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn enqueue(&self, value: ItemType) {
        self.buffer
            .update_while_locked(|buffer| buffer.push_back(value));
    }
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn new() -> Self {
        Self {
            buffer: ARCLock::new(VecDeque::new()),
        }
    }
}

impl<ItemType> Clone for ThreadSafeQueue<ItemType> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
        }
    }
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn dequeue(&self) -> Option<ItemType> {
        self.buffer.update_while_locked(|buffer| buffer.pop_front())
    }
}
