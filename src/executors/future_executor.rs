use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use crate::shared::{pair, Suspender, Task};

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
        let future = Task::from_ref(&mut future);
        let (suspender, waker) = waker_pair;
        let mut context: Context<'_> = Context::from_waker(waker);
        loop {
            match future.poll_task(&mut context) {
                Poll::Pending => suspender.suspend(),
                Poll::Ready(output) => return output,
            }
        }
    })
}
