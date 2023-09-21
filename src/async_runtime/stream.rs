use futures_lite::Stream;
use parking_lot::Mutex;
use std::ops::{Deref, DerefMut};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone)]
pub struct AsyncStream<ItemType> {
    inner: Arc<Mutex<Inner<ItemType>>>,
    started: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn insert_item(&mut self, value: ItemType) {
        self.started = true;
        let inners = self.inner.clone();
        let mut inner_lock = inners.lock();
        inner_lock.count += 1;
        inner_lock.buffer.push_back(value);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn new() -> Self {
        AsyncStream::<ItemType> {
            inner: Arc::new(Mutex::new(Inner::new())),
            started: false,
        }
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
        let inner_lock = self.inner.clone();
        let waker_clone = cx.waker().clone();
        let mut inner_lock = inner_lock.lock();
        if !inner_lock.buffer.is_empty() {
            return Poll::Ready(inner_lock.buffer.pop_front());
        } else if inner_lock.buffer.is_empty() {
            return Poll::Ready(None);
        }
        if inner_lock.count != 0 {
            let Some(value) = inner_lock.buffer.pop_front() else {
                waker_clone.wake_by_ref();
                return Poll::Pending;
            };
            inner_lock.count -= 1;
            return Poll::Ready(Some(value));
        }
        Poll::Ready(None)
    }
}

unsafe impl<ItemType> Send for AsyncStream<ItemType> {}

unsafe impl<ItemType> Sync for AsyncStream<ItemType> {}

pub(crate) struct Inner<T> {
    pub(crate) buffer: VecDeque<T>,
    pub(crate) count: usize,
}

impl<ItemType> Inner<ItemType> {
    pub(crate) fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
            count: 0,
        }
    }
}
