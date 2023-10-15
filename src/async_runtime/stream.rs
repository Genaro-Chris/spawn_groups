use futures_lite::Stream;
use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

#[derive(Clone)]
pub(super) struct AsyncStream<ItemType> {
    count: Arc<AtomicUsize>,
    inner: Arc<Mutex<Inner<ItemType>>>,
    started: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn insert_item(&mut self, value: ItemType) {
        self.started = true;
        let mut inner_lock: MutexGuard<'_, RawMutex, Inner<ItemType>> = self.inner.lock();
        self.count.fetch_add(1, Ordering::SeqCst);
        inner_lock.buffer.push_back(value);
    }
}

impl<ItemType> Default for AsyncStream<ItemType> {
    fn default() -> Self {
        AsyncStream::<ItemType> {
            count: Arc::new(AtomicUsize::new(0)),
            inner: Arc::new(Mutex::new(Inner::default())),
            started: false,
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    fn decrement_count(&self) {
        self.count.fetch_min(1, Ordering::SeqCst);
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner_lock: MutexGuard<'_, RawMutex, Inner<ItemType>> = self.inner.lock();
        if inner_lock.buffer.is_empty() {
            return Poll::Ready(None);
        }
        if self.count() != 0 {
            let Some(value) = inner_lock.buffer.pop_front() else {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            };
            self.decrement_count();
            return Poll::Ready(Some(value));
        }
        Poll::Ready(None)
    }
}

struct Inner<T> {
    buffer: VecDeque<T>,
}

impl<T> Default for Inner<T> {
    fn default() -> Self {
        Self {
            buffer: VecDeque::default(),
        }
    }
}
