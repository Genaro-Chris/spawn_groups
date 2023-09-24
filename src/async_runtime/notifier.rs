use std::sync::{Mutex, Condvar};
use cooked_waker::WakeRef;

#[derive(Debug, Default)] 
pub struct Notifier {
    was_notified: Mutex<bool>,
    cv: Condvar,
}

impl WakeRef for Notifier {
    fn wake_by_ref(&self) {
        let was_notified = {
            let Ok(mut lock) = self.was_notified.lock() else {
                return;
            };
            std::mem::replace(&mut *lock, true)
        };
        if !was_notified {
            self.cv.notify_one();
        }
    }
}