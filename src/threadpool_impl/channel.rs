use std::{
    collections::BinaryHeap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Condvar,
    },
};

use crate::shared::mutex::StdMutex;

pub(crate) struct Channel<T: Ord> {
    inner: Inner<T>,
}

impl<T: Ord> Channel<T> {
    pub(crate) fn enqueue(&self, value: T) {
        self.inner.enqueue(value)
    }
}

impl<T: Ord> Channel<T> {
    pub(crate) fn new() -> Self {
        Self {
            inner: Inner::new(),
        }
    }
}

impl<T: Ord> Channel<T> {
    pub(crate) fn dequeue(&self) -> Option<T> {
        self.inner.dequeue()
    }
}

impl<T: Ord> Channel<T> {
    pub(crate) fn clear(&self) {
        self.inner.clear()
    }

    pub(crate) fn end(&self) {
        self.inner.end()
    }
}

struct Inner<T: Ord> {
    mtx: StdMutex<BinaryHeap<T>>,
    cvar: Condvar,
    closed: AtomicBool,
}

impl<T: Ord> Inner<T> {
    fn new() -> Self {
        Self {
            mtx: StdMutex::new(BinaryHeap::new()),
            cvar: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }

    fn enqueue(&self, value: T) {
        let mut lock = self.mtx.lock();
        lock.push(value);
        self.cvar.notify_one();
    }

    fn dequeue(&self) -> Option<T> {
        let mut lock = self.mtx.lock();
        while lock.is_empty() {
            if self.closed.load(Ordering::Relaxed) {
                return None;
            }
            lock = self.cvar.wait(lock).unwrap();
        }
        lock.pop()
    }

    fn clear(&self) {
        self.mtx.lock().clear();
    }

    fn end(&self) {
        let mut lock = self.mtx.lock();
        self.closed.store(true, Ordering::Relaxed);
        lock.clear();
        self.cvar.notify_all();
    }
}
