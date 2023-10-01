use std::{
    future::Future,
    time::{Duration, Instant},
};

pub struct Delay {
    duration: Duration,
    now: Instant,
}

impl Delay {
    pub fn new(duration: Duration) -> Self {
        Delay {
            duration,
            now: Instant::now(),
        }
    }
}

pub(crate) fn sleep_for(duration: Duration) -> Delay {
    Delay::new(duration)
}

impl Future for Delay {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.now.elapsed() >= self.duration {
            return std::task::Poll::Ready(());
        }
        cx.waker().wake_by_ref();
        std::task::Poll::Pending
    }
}
