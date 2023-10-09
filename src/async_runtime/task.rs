use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    task::Poll,
};

type LocalBoxedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct Task {
    pub(crate) future: Arc<Mutex<LocalBoxedFuture>>,
    pub(crate) complete: Arc<AtomicBool>,
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

    fn complete(&self) {
        self.complete.store(true, Ordering::Release);
    }
}

impl Future for Task {
    type Output = ();
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let task = self.clone();
        let mut future: MutexGuard<'_, RawMutex, Pin<Box<dyn Future<Output = ()> + Send>>> =
            task.future.lock();
        return match future.as_mut().poll(cx) {
            Poll::Ready(_) => {
                self.complete();
                Poll::Ready(())
            }
            Poll::Pending => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        };
    }
}
