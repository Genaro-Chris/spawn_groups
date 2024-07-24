use std::task::{Context, Poll, Waker};

use crate::async_runtime::task::Task;

use super::{
    parker::{pair, Parker},
    waker::waker_helper,
};

fn parker_and_waker() -> (Parker, Waker) {
    let (parker, unparker) = pair();
    let waker = waker_helper(move || {
        unparker.unpark();
    });
    (parker, waker)
}

pub(crate) fn block_task(task: Task) {
    if task.is_completed() {
        return;
    }

    let (parker, waker) = parker_and_waker();
    let mut context: Context<'_> = Context::from_waker(&waker);
    let Ok(mut future) = task.lock() else {
        return;
    };
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => parker.park(),
        }
    }
}
