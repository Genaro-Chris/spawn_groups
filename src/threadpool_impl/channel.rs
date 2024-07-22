use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
};

pub struct Channel<ItemType> {
    pair: Arc<(Mutex<VecDeque<ItemType>>, Condvar)>,
    closed: Arc<AtomicBool>,
}

impl<ItemType> Channel<ItemType> {
    pub(crate) fn enqueue(&self, value: ItemType) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        let Ok(mut lock) = self.pair.0.lock() else {
            return;
        };
        lock.push_back(value);
        if lock.len() == 1 {
            self.pair.1.notify_one();
        }
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
    pub(crate) fn dequeue(&self) -> Option<ItemType> {
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
    pub fn close(&self) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(_lock) = self.pair.0.lock() {
            self.closed.store(true, Ordering::Relaxed);
            self.pair.1.notify_all();
        }
    }

    pub(crate) fn clear(&self) {
        let Ok(mut lock) = self.pair.0.lock() else {
            return;
        };
        lock.clear();
    }
}

impl<ItemType> Iterator for Channel<ItemType> {
    type Item = ItemType;

    fn next(&mut self) -> Option<Self::Item> {
        self.dequeue()
    }
}
