use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use async_std::stream::Stream;
use async_std::sync::Mutex;
use async_std::task::Builder;

use super::inner::Inner;
pub(crate) type AsyncIterator<ItemType> = dyn Stream<Item = ItemType>;

pub struct AsyncStream<ItemType> {
    inner: Arc<Mutex<Inner<ItemType>>>,
    started: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn insert_item(&mut self, value: ItemType) {
        self.started = true;
        let inners = self.inner.clone();
        let mut inner_lock = inners.lock().await;
        inner_lock.buffer.push_back(value);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn increment(&mut self) {
        let inners = self.inner.clone();
        Builder::new().blocking(async move {
            let mut inner_lock = inners.lock().await;
            inner_lock.count += 1;
            inner_lock.increment_task_count();
        });
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub async fn first(&self) -> Option<ItemType> {
        let inners = self.inner.clone();
        let mut buffer_lock = inners.lock().await;
        buffer_lock.buffer.pop_front()
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn buffer_count(&self) -> usize {
        let inner_lock = self.inner.clone();
        let inner_lock = inner_lock.lock().await;
        inner_lock.buffer.len()
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn task_count(&self) -> usize {
        let inner_lock = self.inner.clone();
        async_std::task::block_on(async move {
            let inner_lock = inner_lock.lock().await;
            inner_lock.task_count()
        })
    }

    pub(crate) async fn decrement_task_count(&mut self) {
        let inner_lock = self.inner.clone();
        let mut inner_lock = inner_lock.lock().await;
        inner_lock.decrement_task_count();
    }

    pub(crate) fn cancel_tasks(&self) {
        let inner_lock = self.inner.clone();
        async_std::task::block_on(async move {
            let mut inner_lock = inner_lock.lock().await;
            inner_lock.cancel_tasks();
        });
    }
}

impl<ItemType> Clone for AsyncStream<ItemType> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            started: self.started,
        }
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
        let builder = Builder::new().name(String::from("Builder"));
        builder.blocking(async move {
            let mut inner_lock = inner_lock.lock().await;
            if inner_lock.cancelled && !inner_lock.buffer.is_empty() {
                return Poll::Ready(inner_lock.buffer.pop_front());
            } else if inner_lock.cancelled && inner_lock.buffer.is_empty() {
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
        })
    }
}

unsafe impl<ItemType> Send for AsyncStream<ItemType> {}

unsafe impl<ItemType> Sync for AsyncStream<ItemType> {}
