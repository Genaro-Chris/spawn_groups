use crate::shared::Task;

use super::{pair, Suspender};
use std::{
    sync::Arc,
    task::{Context, Poll, Waker},
};

thread_local! {
    pub(crate) static WAKER_PAIR: (Arc<Suspender>, Waker) = {
        pair()
    };
}

#[inline]
pub(crate) fn block_on<T>(future: Task<T>, pair: &(Arc<Suspender>, Waker)) -> T {
    let future = Task::new(future);
    let mut context: Context<'_> = Context::from_waker(&pair.1);
    loop {
        match future.poll_task(&mut context) {
            Poll::Pending => pair.0.suspend(),
            Poll::Ready(output) => return output,
        }
    }
}
