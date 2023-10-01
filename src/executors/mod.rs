use std::future::Future;

use crate::async_runtime::task::Task;

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
    let (notifier, waker) =
        local_executor::WAKER_PAIR.with(|waker| (waker.0.clone(), waker.1.clone()));
    local_executor::block_future(future, notifier, waker)
}

pub fn block_task(task: Task) {
    let (notifier, waker) =
        local_executor::WAKER_PAIR.with(|waker| (waker.0.clone(), waker.1.clone()));
    task_executor::block_task(task, notifier, waker)
}
