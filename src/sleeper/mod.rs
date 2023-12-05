mod delay;

use std::time::Duration;

use self::delay::Delay;

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// This function is an async version of ``std::thread::sleep``.
///
/// Example
///
/// ```rust
/// use spawn_groups::{block_on, sleep};
/// use std::time::Duration;
///
/// futures_lite::future::block_on(async {
///     sleep(Duration::from_secs(2)).await;
/// });
/// ```
pub fn sleep(duration: Duration) -> Delay {
    Delay::new(duration)
}
