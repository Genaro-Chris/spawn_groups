use cooked_waker::WakeRef;
use std::sync::{Condvar, Mutex, MutexGuard};

#[derive(Default)]
pub struct Notifier {
    was_notified: Mutex<bool>,
    cv: Condvar,
}

impl WakeRef for Notifier {
    fn wake_by_ref(&self) {
        let was_notified: bool = {
            let mut lock: MutexGuard<'_, bool> = self.was_notified.lock().unwrap();
            std::mem::replace(&mut *lock, true)
        };
        if !was_notified {
            self.cv.notify_one();
        }
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
