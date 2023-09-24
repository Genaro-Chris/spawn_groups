use futures_lite::Stream;
use parking_lot::Mutex;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicUsize;
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone)]
pub struct AsyncStream<ItemType> {
    count: Arc<AtomicUsize>,
    inner: Arc<Mutex<Inner<ItemType>>>,
    started: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn insert_item(&mut self, value: ItemType) {
        self.started = true;
        let mut inner_lock = self.inner.lock();
        self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        inner_lock.buffer.push_back(value);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn new() -> Self {
        AsyncStream::<ItemType> {
            count: Arc::new(AtomicUsize::new(0)),
            inner: Arc::new(Mutex::new(Inner::new())),
            started: false,
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    fn count(&self) -> usize {
        self.count.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn decrement_count(&self) {
        self.count.fetch_min(1, std::sync::atomic::Ordering::SeqCst);
    }
}

impl<ItemType: 'static> Deref for AsyncStream<ItemType> {
    type Target = dyn Stream<Item = ItemType>;
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl<ItemType: 'static> DerefMut for AsyncStream<ItemType> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner_lock = self.inner.lock();
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

unsafe impl<ItemType> Send for AsyncStream<ItemType> {}

struct Inner<T> {
    buffer: VecDeque<T>,
}

impl<ItemType> Inner<ItemType> {
    fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }
}
