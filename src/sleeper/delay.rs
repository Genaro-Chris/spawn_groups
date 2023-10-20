use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Delay {
    duration: Duration,
    now: Instant,
}

impl Delay {
    pub(crate) fn new(duration: Duration) -> Self {
        Delay {
            duration,
            now: Instant::now(),
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.now.elapsed() >= self.duration {
            true => Poll::Ready(()),
            false => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
