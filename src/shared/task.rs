use std::{
    future::Future,
    panic::{RefUnwindSafe, UnwindSafe},
    pin::Pin,
    task::{Context, Poll},
};

type LocalFuture<T> = dyn Future<Output = T>;

pub(crate) struct Task<T> {
    future: Pin<Box<LocalFuture<T>>>,
}

impl<T> Task<T> {
    pub(crate) fn new<Fut: Future<Output = T> + 'static>(future: Fut) -> Self {
        Self {
            future: unsafe { Pin::new_unchecked(Box::new(future)) },
        }
    }

    #[inline]
    pub(crate) fn poll_task(&mut self, cx: &mut Context<'_>) -> Poll<T> {
        self.future.as_mut().poll(cx)
    }
}

impl<T> UnwindSafe for Task<T> {}
impl<T> RefUnwindSafe for Task<T> {}
unsafe impl<T> Send for Task<T> {}
unsafe impl<T> Sync for Task<T> {}
