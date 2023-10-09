use std::{
    future::Future,
    sync::Arc,
    task::{Context, Waker},
};

use cooked_waker::IntoWaker;

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Notifier>, Waker) = {
        let notifier = Arc::new(Notifier::default());
        let waker = notifier.clone().into_waker().clone();
        (notifier, waker)
    };
}

use crate::{
    async_runtime::{notifier::Notifier, task::Task},
    pin_future,
};

#[inline]
pub(crate) fn block_task(task: Task, notifier: Arc<Notifier>, waker: &Waker) {
    if task.is_completed() {
        return;
    }
    pin_future!(task);
    let mut context: Context<'_> = Context::from_waker(waker);
    loop {
        match task.as_mut().poll(&mut context) {
            std::task::Poll::Pending => notifier.wait(),
            std::task::Poll::Ready(_) => return,
        }
    }
}
