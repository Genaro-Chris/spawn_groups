use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use crate::shared::{pair, Suspender};

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
#[inline]
pub fn block_on<Fut: Future>(future: Fut) -> Fut::Output {
    thread_local! {
        static PAIR: (Arc<Suspender>, Waker) = {
            pair()
        };
    }

    PAIR.with(move |waker_pair| {
        let mut future = future;
        let mut future = unsafe { Pin::new_unchecked(&mut future) };
        let (suspender, waker) = &*waker_pair;
        let mut context: Context<'_> = Context::from_waker(waker);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Pending => suspender.suspend(),
                Poll::Ready(output) => return output,
            }
        }
    })
}
