use futures_lite::Stream;
use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex};
use std::{
    collections::VecDeque,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Clone)]
pub(super) struct AsyncStream<ItemType> {
    buffer: Arc<Mutex<VecDeque<ItemType>>>,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn insert_item(&self, value: ItemType) {
        let mut inner_lock: MutexGuard<'_, RawMutex, VecDeque<ItemType>> = self.buffer.lock();
        inner_lock.push_back(value);
    }
}

impl<ItemType> Default for AsyncStream<ItemType> {
    fn default() -> Self {
        AsyncStream::<ItemType> {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner_lock: MutexGuard<'_, RawMutex, VecDeque<ItemType>> = self.buffer.lock();
        if inner_lock.is_empty() {
            return Poll::Ready(None);
        }
        let Some(value) = inner_lock.pop_front() else {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        };
        Poll::Ready(Some(value))
    }
}
