use std::{
    future::Future,
    sync::Arc,
    task::{Context, Waker},
};

use cooked_waker::IntoWaker;

use crate::{async_runtime::notifier::Notifier, pin_future};

thread_local! {
    pub static WAKER_PAIR: (Arc<Notifier>, Waker) = {
        let notifier = Arc::new(Notifier::default());
        (notifier.clone(), notifier.into_waker())
    };
}

pub(crate) fn block_future<Fut: Future>(
    future: Fut,
    notifier: Arc<Notifier>,
    waker: Waker,
) -> Fut::Output {
    pin_future!(future);
    let mut context = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => notifier.wait(),
        }
    }
}
