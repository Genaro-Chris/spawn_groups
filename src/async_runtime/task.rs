use parking_lot::Mutex;
use std::{
    future::Future,
    pin::Pin,
    sync::{atomic::AtomicBool, Arc},
    task::Poll,
};

type LocalBoxedFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct Task {
    pub(crate) future: Arc<Mutex<LocalBoxedFuture>>,
    pub(crate) complete: Arc<AtomicBool>,
}

impl Task {
    pub fn is_completed(&self) -> bool {
        self.complete.load(std::sync::atomic::Ordering::Acquire)
    }

    fn complete(&self) {
        self.complete
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

impl Future for Task {
    type Output = ();
    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let task = self.clone();
        let mut future = task.future.lock();
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
