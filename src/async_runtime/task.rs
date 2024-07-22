use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

type LocalBoxedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct Task {
    future: Arc<Mutex<LocalBoxedFuture>>,
    complete: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

impl Task {
    pub(crate) fn new<Fut: Future<Output = ()> + Send + 'static>(fut: Fut) -> Self {
        Self {
            future: Arc::new(Mutex::new(Box::pin(fut))),
            complete: Arc::new(AtomicBool::new(false)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.complete.load(Ordering::Relaxed)
    }

    pub(crate) fn complete(&self) {
        self.complete.store(true, Ordering::Release);
    }

    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

impl Deref for Task {
    type Target = Arc<Mutex<LocalBoxedFuture>>;

    fn deref(&self) -> &Self::Target {
        &self.future
    }
}

impl Future for Task {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.future.lock().unwrap().as_mut().poll(cx) {
            Poll::Ready(()) => {
                self.complete();
                Poll::Ready(())
            }
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }
}
