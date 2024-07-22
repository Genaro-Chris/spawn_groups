use std::{sync::Arc, task::Waker};

use super::waker_traits::{IntoWaker, WakeRef};

struct WakerHelper<F>(F);

pub(crate) fn waker_helper<F: Fn() + 'static>(f: F) -> Waker {
    Arc::new(WakerHelper(f)).into_waker()
}

impl<F: Fn() + 'static> WakeRef for WakerHelper<F> {
    fn wake_by_ref(&self) {
        (self.0)();
    }
}
