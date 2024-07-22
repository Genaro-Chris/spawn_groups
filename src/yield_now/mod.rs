use crate::yield_now::yielder::Yielder;

mod yielder;

/// Wakes the current task and returns [`std::task::Poll::Pending`] once.
///
/// This function is useful when we want to cooperatively give time to the task executor. It is
/// generally a good idea to yield inside loops because that way we make sure long-running tasks
/// don't prevent other tasks from running.
///
/// # Examples
/// ```
/// use spawn_groups::{block_on, yield_now};
/// block_on(async {
///     yield_now().await;
/// });
/// ```
pub fn yield_now() -> Yielder {
    Yielder::default()
}

/// Resolves to the provided value.
///
/// # Examples
/// ```
/// use spawn_groups::{block_on, ready};
/// block_on(async {
///     let ten = ready(10).await;
///     assert_eq!(ten, 10);
/// });
/// ```
pub async fn ready<ValueType>(val: ValueType) -> ValueType {
    val
}
