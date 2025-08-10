use std::{
    future::Future,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

pub(crate) struct Task<T> {
    poll_fn: fn(NonNull<()>, &mut Context<'_>) -> Poll<T>,
    drop_fn: fn(NonNull<()>),
    raw_ptr: NonNull<()>,
}

impl<T> Task<T> {
    pub(crate) fn new<Fut: Future<Output = T>>(fut: Fut) -> Self {
        unsafe {
            Self {
                raw_ptr: NonNull::new_unchecked(Box::into_raw(Box::new(fut)).cast()),
                poll_fn: |self_ptr, cntx| {
                    Pin::new_unchecked(self_ptr.cast::<Fut>().as_mut()).poll(cntx)
                },
                drop_fn: |raw_ptr| _ = Box::from_raw(raw_ptr.cast::<Fut>().as_ptr()),
            }
        }
    }

    pub(crate) fn poll_task(&self, cx: &mut Context<'_>) -> Poll<T> {
        (self.poll_fn)(self.raw_ptr, cx)
    }
}

impl<T> Unpin for Task<T> {}

impl<T> Future for Task<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.poll_task(cx)
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        (self.drop_fn)(self.raw_ptr)
    }
}
