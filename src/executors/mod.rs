use std::{future::Future, sync::Arc, task::Waker};

use cooked_waker::IntoWaker;

use crate::async_runtime::{notifier::Notifier, task::Task};

use self::{local_executor::block_future, task_executor::block_on_task};

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
    let waker_pair: Result<(Arc<Notifier>, Waker), std::thread::AccessError> =
        local_executor::WAKER_PAIR
            .try_with(|waker_pair: &(Arc<Notifier>, Waker)| waker_pair.clone());
    match waker_pair {
        Ok((notifier, waker)) => block_future(future, notifier, &waker),
        Err(_) => {
            let notifier: Arc<Notifier> = Arc::new(Notifier::default());
            let waker: Waker = notifier.clone().into_waker();
            block_future(future, notifier, &waker)
        }
    }
}

pub(crate) fn block_task(task: Task) {
    let waker_pair: Result<(Arc<Notifier>, Waker), std::thread::AccessError> =
        local_executor::WAKER_PAIR
            .try_with(|waker_pair: &(Arc<Notifier>, Waker)| waker_pair.clone());
    match waker_pair {
        Ok((notifier, waker)) => block_on_task(task, notifier, &waker),
        Err(_) => {
            let notifier: Arc<Notifier> = Arc::new(Notifier::default());
            let waker: Waker = notifier.clone().into_waker();
            block_on_task(task, notifier, &waker)
        }
    }
}
