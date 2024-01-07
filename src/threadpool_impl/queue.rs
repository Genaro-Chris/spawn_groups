use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(crate) struct ThreadSafeQueue<ItemType> {
    buffer: Arc<Mutex<VecDeque<ItemType>>>,
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn enqueue(&self, value: ItemType) {
        if let Ok(mut lock) = self.buffer.lock() {
            lock.push_back(value);
        }
    }
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
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
        let Ok(mut buffer_lock) = self.buffer.lock() else {
            return None;
        };
        buffer_lock.pop_front()
    }
}
