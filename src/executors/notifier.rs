#![allow(dead_code)]

use std::sync::{Condvar, Mutex, MutexGuard};

use crate::executors::waker_traits::WakeRef;

#[derive(Default)]
pub(crate) struct Notifier {
    was_notified: Mutex<bool>,
    cv: Condvar,
}

impl Notifier {
    pub(crate) fn wake(&self) {
        let mut was_notified: MutexGuard<'_, bool> = self.was_notified.lock().unwrap();

        let was_notified: bool =
            { std::mem::replace(&mut was_notified, true) };
        if !was_notified {
            self.cv.notify_one();
        }
    }
}

impl WakeRef for Notifier {
    fn wake_by_ref(&self) {
        self.wake()
    }
}

impl Notifier {
    pub(crate) fn wait(&self) {
        let mut was_notified: MutexGuard<'_, bool> = self.was_notified.lock().unwrap();

        while !*was_notified {
            was_notified = self.cv.wait(was_notified).unwrap();
        }
        *was_notified = false;
    }
}

