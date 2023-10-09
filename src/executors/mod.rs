use std::{future::Future, sync::Arc, task::Waker};

use crate::async_runtime::{notifier::Notifier, task::Task};

mod local_executor;
mod task_executor;

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
    local_executor::WAKER_PAIR.with(|waker| {
        let notifier: Arc<Notifier> = waker.0.clone();
        let waker: Waker = waker.1.clone();
        local_executor::block_future(future, notifier, &waker)
    })
}

pub(crate) fn block_task(task: Task) {
    task_executor::WAKER_PAIR.with(|waker| {
        let notifier: Arc<Notifier> = waker.0.clone();
        let waker: Waker = waker.1.clone();
        task_executor::block_task(task, notifier, &waker)
    });
}
