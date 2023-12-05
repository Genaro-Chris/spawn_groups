use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use super::queueorder::QueueOrder;

pub struct ThreadSafeQueue<ItemType> {
    buffer: Arc<Mutex<VecDeque<ItemType>>>,
    order: QueueOrder,
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn enqueue(&self, value: ItemType) {
        if let Ok(mut lock) = self.buffer.lock() {
            lock.push_back(value);
        }
    }
}

impl<ItemType> Clone for ThreadSafeQueue<ItemType> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            order: self.order,
        }
    }
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn new(order: QueueOrder) -> Self {
        ThreadSafeQueue::<ItemType> {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            order,
        }
    }
}

impl<ItemType> ThreadSafeQueue<ItemType> {
    pub fn dequeue(&self) -> Option<ItemType> {
        let Ok(mut buffer_lock) = self.buffer.lock() else {
            return None;
        };
        match self.order {
            QueueOrder::FirstOut => buffer_lock.pop_front(),
            QueueOrder::LastOut => buffer_lock.pop_back(),
        }
    }
}
