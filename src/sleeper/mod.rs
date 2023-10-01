mod delay;

use std::time::Duration;

use crate::sleeper::delay::sleep_for;

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// This function is an async version of ``std::thread::sleep``.
pub async fn sleep(duration: Duration) {
    sleep_for(duration).await
}
