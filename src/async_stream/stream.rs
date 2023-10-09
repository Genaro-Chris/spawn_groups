use std::sync::Arc;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

use async_mutex::{Mutex, MutexGuard};
use futures_lite::{Stream, StreamExt};

use crate::executors::block_on;

use super::inner::Inner;

pub struct AsyncStream<ItemType> {
    inner: Arc<Mutex<Inner<ItemType>>>,
    started: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn insert_item(&mut self, value: ItemType) {
        self.started = true;
        self.inner.lock().await.buffer.push_back(value);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn buffer_count(&self) -> usize {
        self.inner.lock().await.buffer.len()
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn increment(&mut self) {
        let mut inner_lock: MutexGuard<'_, Inner<ItemType>> = self.inner.lock().await;
        inner_lock.increment_count();
        inner_lock.increment_task_count();
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn first(&self) -> Option<ItemType> {
        let mut cloned: AsyncStream<ItemType> = self.clone();
        cloned.next().await
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn task_count(&self) -> usize {
        block_on(async move { self.inner.lock().await.task_count() })
    }

    pub(crate) async fn decrement_task_count(&mut self) {
        self.inner.lock().await.decrement_task_count();
    }

    pub(crate) fn cancel_tasks(&self) {
        block_on(async move {
            self.inner.lock().await.cancel_tasks();
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

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        block_on(async move {
            let mut inner_lock: MutexGuard<'_, Inner<ItemType>> = self.inner.lock().await;
            if inner_lock.is_cancelled() && inner_lock.buffer.is_empty() {
                return Poll::Ready(None);
            }
            if inner_lock.count() != 0 {
                let Some(value) = inner_lock.buffer.pop_front() else {
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                };
                inner_lock.decrement_count();
                return Poll::Ready(Some(value));
            }
            Poll::Ready(None)
        })
    }
}
