use std::{
    cell::RefCell,
    task::{Context, Poll, Waker},
};

use crate::shared::priority_task::PrioritizedTask;

use super::{pair, Suspender};

thread_local! {
    pub(crate) static WAKER_PAIR: RefCell<(Suspender, Waker)> = {
        RefCell::new(pair())
    };
}

pub(crate) fn block_task(task: PrioritizedTask) {
    if task.is_completed() {
        return;
    }

    WAKER_PAIR.with(move |waker_pair| {
        let mut task = task;
        match waker_pair.try_borrow_mut() {
            Ok(waker_pair) => {
                let (suspender, waker) = &*waker_pair;
                let mut context: Context<'_> = Context::from_waker(waker);
                loop {
                    let Poll::Ready(()) = task.poll_task(&mut context) else {
                        suspender.suspend();
                        continue;
                    };
                    return;
                }
            }
            Err(_) => {
                let (suspender, waker) = pair();
                let mut context: Context<'_> = Context::from_waker(&waker);
                loop {
                    let Poll::Ready(()) = task.poll_task(&mut context) else {
                        suspender.suspend();
                        continue;
                    };
                    return;
                }
            }
        };
    });
}
