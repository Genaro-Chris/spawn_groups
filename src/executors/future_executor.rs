use std::{
    cell::RefCell,
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{executors::waker::waker_helper, pin_future};

use super::parker::{pair, Parker};

fn parker_and_waker() -> (Parker, Waker) {
    let (parker, unparker) = pair();
    let waker = waker_helper(move || {
        unparker.unpark();
    });
    (parker, waker)
}

/// Blocks the current thread until the future is polled to finish.
///
/// Example
/// ```rust
/// let result = spawn_groups::block_on(async {
///     println!("This is an async executor");
///     1
/// });
/// assert_eq!(result, 1);
/// ```
///
pub fn block_on<Fut: Future>(future: Fut) -> Fut::Output {
    pin_future!(future);
    thread_local! {
        static WAKER_PAIR: RefCell<(Parker, Waker)> = {
            RefCell::new(parker_and_waker())
        };
    }
    return WAKER_PAIR.with(|waker_pair| match waker_pair.try_borrow_mut() {
        Ok(waker_pair) => {
            let (parker, waker) = &*waker_pair;
            let mut context: Context<'_> = Context::from_waker(waker);
            loop {
                match future.as_mut().poll(&mut context) {
                    Poll::Ready(output) => return output,
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
            loop {
                match future.as_mut().poll(&mut context) {
                    Poll::Ready(output) => return output,
                    Poll::Pending => parker.park(),
                }
            }
        }
    });
}
