use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
};

#[derive(Default)]
pub struct Channel<ItemType> {
    pair: Arc<(Mutex<VecDeque<ItemType>>, Condvar)>,
    closed: Arc<AtomicBool>,
}

impl<ItemType> Channel<ItemType> {
    pub fn enqueue(&self, value: ItemType) -> bool {
        if self.closed.load(Ordering::Relaxed) {
            return false;
        }
        if let Ok(mut lock) = self.pair.0.lock() {
            lock.push_back(value);
            self.pair.1.notify_one();
            return true;
        }
        false
    }
}

impl<ItemType> Channel<ItemType> {
    pub fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(VecDeque::new()), Condvar::new())),
            closed: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> Clone for Channel<ItemType> {
    fn clone(&self) -> Self {
        Self {
            pair: self.pair.clone(),
            closed: self.closed.clone(),
        }
    }
}

impl<ItemType> Channel<ItemType> {
    pub fn dequeue(&self) -> Option<ItemType> {
        if self.closed.load(Ordering::Relaxed) {
            return None;
        }
        let Ok(mut lock) = self.pair.0.lock() else {
            return None;
        };
        while lock.is_empty() {
            if self.closed.load(Ordering::Relaxed) {
                return None;
            }
            lock = self.pair.1.wait(lock).unwrap();
        }
        lock.pop_front()
    }
}

impl<ItemType> Channel<ItemType> {
    ///
    pub fn close(&self) {
        if let Ok(_lock) = self.pair.0.lock() {
            self.closed.store(true, Ordering::Relaxed);
            self.pair.1.notify_all();
        }
    }

    pub fn clear(&self) {
        if let Ok(mut lock) = self.pair.0.lock() {
            lock.clear();
        }
    }
}

impl<ItemType> Iterator for Channel<ItemType> {
    type Item = ItemType;

    fn next(&mut self) -> Option<Self::Item> {
        self.dequeue()
    }
}
