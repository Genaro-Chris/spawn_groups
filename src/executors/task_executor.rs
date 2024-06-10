use std::{
    sync::Arc,
    task::{Context, Waker},
};

use crate::async_runtime::{notifier::Notifier, task::Task};
use cooked_waker::IntoWaker;

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Notifier>, Waker) = {
        let notifier = Arc::new(Notifier::default());
        let waker = notifier.clone().into_waker();
        (notifier, waker)
    };
}

pub(crate) fn block_on_task(task: Task, notifier: Arc<Notifier>, waker: &Waker) {
    if task.is_completed() {
        return;
    }
    let mut context: Context<'_> = Context::from_waker(waker);
    let Ok(mut task) = task.future.lock() else {
        return;
    };
    loop {
        match task.as_mut().poll(&mut context) {
            std::task::Poll::Ready(()) => return,
            std::task::Poll::Pending => notifier.wait(),
        }
    }
}
