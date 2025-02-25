use super::{pair, Suspender};
use std::{sync::Arc, task::Waker};

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Suspender>, Waker) = {
        pair()
    };
}
