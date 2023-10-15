use std::{
    future::Future,
    sync::Arc,
    task::{Context, Waker},
};

use cooked_waker::IntoWaker;

use crate::{async_runtime::notifier::Notifier, pin_future};

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Notifier>, Waker) = {
        let notifier = Arc::new(Notifier::default());
        let waker = notifier.clone().into_waker();
        (notifier, waker)
    };
}

#[inline]
pub(crate) fn block_future<Fut: Future>(
    future: Fut,
    notifier: Arc<Notifier>,
    waker: &Waker,
) -> Fut::Output {
    let mut context: Context<'_> = Context::from_waker(waker);
    pin_future!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => return output,
            std::task::Poll::Pending => notifier.wait(),
        }
    }
}
