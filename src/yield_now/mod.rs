use crate::yield_now::yielder::Yielder;

mod yielder;

/// Wakes the current task and returns [`Poll::Pending`] once.
///
/// This function is useful when we want to cooperatively give time to the task executor. It is
/// generally a good idea to yield inside loops because that way we make sure long-running tasks
/// don't prevent other tasks from running.
///
/// # Examples
/// ```
/// spawn_groups::block_on(async {
///     spawn_groups::yield_now().await;
/// });
/// ```
pub fn yield_now() -> Yielder {
    Yielder::default()
}
