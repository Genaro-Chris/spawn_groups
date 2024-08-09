use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex,
    },
};

pub(crate) struct Channel<ItemType> {
    inner: Arc<Inner<ItemType>>,
}

impl<ItemType> Channel<ItemType> {
    pub(crate) fn enqueue(&self, value: ItemType) {
        self.inner.enqueue(value)
    }
}

impl<ItemType> Channel<ItemType> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Inner::new()),
        }
    }
}

impl<ItemType> Clone for Channel<ItemType> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<ItemType> Channel<ItemType> {
    pub(crate) fn dequeue(&self) -> Option<ItemType> {
        self.inner.dequeue()
    }
}

impl<ItemType> Channel<ItemType> {
    pub(crate) fn close(&self) {
        self.inner.close()
    }

    pub(crate) fn clear(&self) {
        self.inner.clear()
    }
}

struct Inner<ItemType> {
    closed: AtomicBool,
    pair: (Mutex<VecDeque<ItemType>>, Condvar),
}

impl<ItemType> Inner<ItemType> {
    fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
            pair: (Mutex::new(VecDeque::new()), Condvar::new()),
        }
    }

    fn enqueue(&self, value: ItemType) {
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

    fn dequeue(&self) -> Option<ItemType> {
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

    fn close(&self) {
        if self.closed.swap(true, Ordering::Relaxed) {
            return;
        }
        let Ok(_lock) = self.pair.0.lock() else {
            return;
        };
        self.pair.1.notify_all();
    }

    fn clear(&self) {
        let Ok(mut lock) = self.pair.0.lock() else {
            return;
        };
        lock.clear();
    }
}
