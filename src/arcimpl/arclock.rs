use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use super::arc::ARC;

#[derive(Default)]
pub struct ARCLock<T> {
    lock: Arc<AtomicUsize>,
    ref_ptr: ARC<T>,
}

impl<T> Clone for ARCLock<T> {
    fn clone(&self) -> Self {
        Self {
            lock: self.lock.clone(),
            ref_ptr: self.ref_ptr.clone(),
        }
    }
}

impl<T> ARCLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            lock: Arc::new(AtomicUsize::new(0)),
            ref_ptr: ARC::new(value),
        }
    }
}

impl<T> ARCLock<T> {
    pub fn lock(&self) {
        loop {
            match self
                .lock
                .compare_exchange(0, 4, Ordering::Acquire, Ordering::Acquire)
            {
                Ok(_) => {
                    self.lock.store(1, Ordering::Release);
                    return;
                }
                Err(_) => {
                    thread::sleep(Duration::from_nanos(300));
                }
            }
        }
    }

    pub fn unlock(&self) {
        loop {
            match self
                .lock
                .compare_exchange(1, 3, Ordering::Acquire, Ordering::Acquire)
            {
                Ok(_) => {
                    self.lock.store(0, Ordering::Release);
                    return;
                }
                Err(_) => {
                    thread::sleep(Duration::from_nanos(300));
                }
            }
        }
    }

    pub fn update_while_locked<U>(&self, closure: impl FnOnce(&mut T) -> U) -> U {
        self.lock();
        let result = closure(unsafe { &mut &mut *(*&self.ref_ptr.get()) });
        self.unlock();
        result
    }
}
