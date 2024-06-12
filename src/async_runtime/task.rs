use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::Poll,
};

type LocalBoxedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct Task {
    pub(crate) future: Arc<Mutex<LocalBoxedFuture>>,
    complete: Arc<AtomicBool>,
}

impl Task {
    pub(crate) fn new<Fut: Future<Output = ()> + Send + 'static>(fut: Fut) -> Self {
        Self {
            future: Arc::new(Mutex::new(Box::pin(fut))),
            complete: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }

    pub(crate) fn complete(&self) {
        self.complete.store(true, Ordering::Relaxed);
    }
}

impl Future for Task {
    type Output = ();
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
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
