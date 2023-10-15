use cooked_waker::WakeRef;
use parking_lot::{lock_api::MutexGuard, Condvar, Mutex, RawMutex};

#[derive(Default)]
pub struct Notifier {
    was_notified: Mutex<bool>,
    cv: Condvar,
}

impl WakeRef for Notifier {
    fn wake_by_ref(&self) {
        let was_notified: bool = {
            let mut lock: MutexGuard<'_, RawMutex, bool> = self.was_notified.lock();
            std::mem::replace(&mut *lock, true)
        };
        if !was_notified {
            self.cv.notify_one();
        }
    }
}

impl Notifier {
    pub(crate) fn wait(&self) {
        let mut was_notified: MutexGuard<'_, RawMutex, bool> = self.was_notified.lock();

        if !*was_notified {
            self.cv.wait(&mut was_notified);
        }
        *was_notified = false;
    }
}
