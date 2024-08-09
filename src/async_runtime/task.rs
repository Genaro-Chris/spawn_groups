use std::{
    future::Future, hint, pin::Pin, rc::Rc, sync::atomic::{AtomicBool, AtomicU8, Ordering}, task::{Context, Poll}
};

type LocalFuture = dyn Future<Output = ()>;

#[derive(Clone)]
pub(crate) struct Task {
    inner: Rc<Inner>,
}

impl Task {
    pub(crate) fn new<Fut: Future<Output = ()> + 'static>(future: Fut) -> Self {
        Self {
            inner: Rc::new(Inner::new(future)),
        }
    }
}

impl Task {
    pub(crate) fn is_completed(&self) -> bool {
        self.inner.complete.load(Ordering::Acquire)
    }

    pub(crate) fn complete(&self) {
        self.inner.complete.store(true, Ordering::Release)
    }

    pub(crate) fn cancel_task(&self) {
        self.inner.cancelled.store(true, Ordering::Release)
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn poll_task(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        // ensures that only this method is polling the future right now regardless of all other cloned tasks
        // basically a lightweight spinlock to prevent data race bugs while polling
        while self
            .inner
            .poll_check
            .compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            hint::spin_loop();
        }

        let result = unsafe { Pin::new_unchecked(&mut (*self.inner.ptr)).poll(cx) };
        if result.is_ready() {
            self.complete();
        }
        self.inner.poll_check.store(0, Ordering::Release);
        result
    }
}

unsafe impl Send for Task {}

struct Inner {
    poll_check: AtomicU8,
    ptr: *mut LocalFuture,
    cancelled: AtomicBool,
    complete: AtomicBool,
}

impl Inner {
    fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Self {
            poll_check: AtomicU8::new(0),
            ptr: Box::into_raw(Box::new(future)),
            complete: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            _ = Box::from_raw(self.ptr);
        }
    }
}
