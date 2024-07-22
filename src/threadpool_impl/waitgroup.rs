use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub(crate) struct WaitGroup {
    pair: Arc<(Mutex<usize>, Condvar)>,
}

impl WaitGroup {
    pub(crate) fn new() -> Self {
        Self {
            pair: Arc::new((Mutex::new(0), Condvar::new())),
        }
    }
}

impl WaitGroup {
    pub(crate) fn enter(&self) {
        let Ok(mut guard) = self.pair.0.lock() else {
            return;
        };
        (*guard) += 1;
    }

    pub(crate) fn leave(&self) {
        let Ok(mut guard) = self.pair.0.lock() else {
            return;
        };
        (*guard) -= 1;
        if (*guard) == 0 {
            self.pair.1.notify_all();
        }
    }

    pub(crate) fn wait(&self) {
        let Ok(mut guard) = self.pair.0.lock() else {
            return;
        };
        while *guard > 0 {
            guard = self.pair.1.wait(guard).unwrap();
        }
    }
}
