use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

#[derive(Debug, Default)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Yielder {
    yield_now: bool,
}

impl Future for Yielder {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.yield_now {
            true => Poll::Ready(()),
            false => {
                self.yield_now = true;
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
