use parking_lot::{Mutex, Condvar};
use cooked_waker::WakeRef;

#[derive(Debug, Default)] 
pub struct Notifier {
    was_notified: Mutex<bool>,
    cv: Condvar,
}

impl WakeRef for Notifier {
    fn wake_by_ref(&self) {
        let was_notified = {
            let mut lock = self.was_notified.lock();
            std::mem::replace(&mut *lock, true)
        };
        if !was_notified {
            self.cv.notify_one();
        }
    }
}

impl Notifier {
    pub(crate) fn wait(&self) {
        let mut was_notified = self.was_notified.lock();

       if !*was_notified {
            self.cv.wait(&mut was_notified);
        }
        *was_notified = false;
    }
}