use std::{
    cell::RefCell,
    task::{Context, Poll, Waker},
};

use crate::{
    async_runtime::task::Task,
    executors::parker::{pair, Parker},
};

use super::waker::waker_helper;

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

    thread_local! {
        static TASK_PAIR: RefCell<(Parker, Waker)> = {
            RefCell::new(parker_and_waker())
        };
    }

    TASK_PAIR.with(|waker_pair| match waker_pair.try_borrow_mut() {
        Ok(waker_pair) => {
            let (parker, ref waker) = &*waker_pair;
            let mut context: Context<'_> = Context::from_waker(waker);
            let Ok(mut task) = task.lock() else {
                return;
            };
            loop {
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(()) => return,
                    Poll::Pending => parker.park(),
                }
            }
        }
        Err(_) => {
            let (parker, unparker) = pair();
            let waker = waker_helper(move || {
                unparker.unpark();
            });
            let mut context: Context<'_> = Context::from_waker(&waker);
            let Ok(mut task) = task.lock() else {
                return;
            };
            loop {
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(()) => return,
                    Poll::Pending => parker.park(),
                }
            }
        }
    });
}
