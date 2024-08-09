use std::{
    cell::RefCell,
    future::Future,
    task::{Context, Poll, Waker},
};

use super::suspender::{pair, Suspender};

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
    let mut future = future;
    let mut future = unsafe { std::pin::Pin::new_unchecked(&mut future) };
    thread_local! {
        static WAKER_PAIR: RefCell<(Suspender, Waker)> = {
            RefCell::new(pair())
        };
    }
    return WAKER_PAIR.with(|waker_pair| match waker_pair.try_borrow_mut() {
        Ok(waker_pair) => {
            let (suspender, waker) = &*waker_pair;
            let mut context: Context<'_> = Context::from_waker(waker);
            loop {
                let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
                    suspender.suspend();
                    continue;
                };
                return output;
            }
        }
        Err(_) => {
            let (suspender, waker) = pair();
            let mut context: Context<'_> = Context::from_waker(&waker);
            loop {
                let Poll::Ready(output) = Future::poll(future.as_mut(), &mut context) else {
                    suspender.suspend();
                    continue;
                };
                return output;
            }
        }
    });
}
