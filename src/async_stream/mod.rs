use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use async_lock::{Mutex, MutexGuard};
use futures_lite::{Stream, StreamExt};

use crate::executors::block_on;

pub struct AsyncStream<ItemType> {
    buffer: Arc<Mutex<VecDeque<ItemType>>>,
    started: bool,
    counts: (Arc<AtomicUsize>, Arc<AtomicUsize>),
    cancelled: bool,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn insert_item(&mut self, value: ItemType) {
        if !self.started {
            self.started = true;
        }
        self.buffer.lock().await.push_back(value);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn buffer_count(&self) -> usize {
        self.buffer.lock().await.len()
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn increment(&self) {
        self.counts.0.fetch_add(1, Ordering::Acquire);
        self.counts.1.fetch_add(1, Ordering::Acquire);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn first(&mut self) -> Option<ItemType> {
        self.next().await
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn task_count(&self) -> usize {
        self.counts.1.load(Ordering::Acquire)
    }

    pub(crate) fn decrement_task_count(&self) {
        if self.task_count() > 0 {
            self.counts.1.fetch_sub(1, Ordering::Acquire);
        }
    }

    pub(crate) fn item_count(&self) -> usize {
        self.counts.0.load(Ordering::Acquire)
    }

    pub(crate) fn decrement_count(&self) {
        if self.item_count() > 0 {
            self.counts.0.fetch_sub(1, Ordering::Acquire);
        }
    }

    pub(crate) fn cancel_tasks(&mut self) {
        self.cancelled = true;
        self.counts.1.store(0, Ordering::Release);
    }
}

impl<ItemType> Clone for AsyncStream<ItemType> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            started: self.started,
            counts: self.counts.clone(),
            cancelled: self.cancelled,
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn new() -> Self {
        AsyncStream::<ItemType> {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            started: false,
            counts: (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0))),
            cancelled: false,
        }
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        block_on(async move {
            let mut inner_lock: MutexGuard<'_, VecDeque<ItemType>> = self.buffer.lock().await;
            if self.cancelled && inner_lock.is_empty() || self.item_count() == 0 {
                return Poll::Ready(None);
            }
            let Some(value) = inner_lock.pop_front() else {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            };
            self.decrement_count();
            Poll::Ready(Some(value))
        })
    }
}
