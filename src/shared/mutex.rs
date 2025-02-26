use std::sync::{Mutex, MutexGuard};

#[derive(Default)]
pub(crate) struct StdMutex<T: ?Sized>(Mutex<T>);
pub(crate) type StdMutexGuard<'a, T> = MutexGuard<'a, T>;

impl<T> StdMutex<T> {
    pub(crate) fn new(t: T) -> Self {
        Self(Mutex::new(t))
    }

    pub(crate) fn lock(&self) -> StdMutexGuard<T> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
