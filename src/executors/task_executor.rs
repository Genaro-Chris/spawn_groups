use std::{
    future::Future,
    sync::Arc,
    task::{Context, Waker},
};

use crate::{
    async_runtime::{notifier::Notifier, task::Task},
    pin_future,
};

use cooked_waker::IntoWaker;

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Notifier>, Waker) = {
        let notifier = Arc::new(Notifier::default());
        let waker = notifier.clone().into_waker();
        (notifier, waker)
    };
}


#[inline]
pub(crate) fn block_on_task(task: Task, notifier: Arc<Notifier>, waker: &Waker) {
    if task.is_completed() {
        return;
    }
    pin_future!(task);
    let mut context: Context<'_> = Context::from_waker(waker);
    loop {
        match task.as_mut().poll(&mut context) {
            std::task::Poll::Ready(()) => return,
            std::task::Poll::Pending => notifier.wait(),
        }
    }
}
