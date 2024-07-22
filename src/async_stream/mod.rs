use std::{
    collections::VecDeque,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use async_mutex::{Mutex, MutexGuard};
use futures_lite::{Stream, StreamExt};

use crate::executors::block_on;

pub struct AsyncStream<ItemType> {
    buffer: Arc<Mutex<VecDeque<ItemType>>>,
    item_count: Arc<AtomicUsize>,
    task_count: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
}

impl<ItemType> AsyncStream<ItemType> {
    #[inline]
    pub(crate) async fn insert_item(&self, value: ItemType) {
        _ = self
            .cancelled
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed);
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
        self.task_count.fetch_add(1, Ordering::SeqCst);
        self.item_count.fetch_add(1, Ordering::SeqCst);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub async fn first(&mut self) -> Option<ItemType> {
        self.next().await
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn task_count(&self) -> usize {
        self.task_count.load(Ordering::Acquire)
    }

    pub(crate) fn decrement_task_count(&self) {
        self.task_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub(crate) fn item_count(&self) -> usize {
        self.item_count.load(Ordering::Acquire)
    }

    pub(crate) fn cancel_tasks(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        self.task_count.store(0, Ordering::Release);
    }
}

impl<ItemType> Clone for AsyncStream<ItemType> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            task_count: self.task_count.clone(),
            item_count: self.item_count.clone(),
            cancelled: self.cancelled.clone(),
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn new() -> Self {
        AsyncStream::<ItemType> {
            buffer: Arc::new(Mutex::new(VecDeque::new())),
            item_count: Arc::new(AtomicUsize::new(0)),
            task_count: Arc::new(AtomicUsize::new(0)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        block_on(async move {
            let mut inner_lock: MutexGuard<'_, VecDeque<ItemType>> = self.buffer.lock().await;
            if self.cancelled.load(Ordering::Relaxed) && inner_lock.is_empty()
                || self.item_count() == 0
            {
                return Poll::Ready(None);
            }
            let Some(value) = inner_lock.pop_front() else {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            };
            self.item_count.fetch_sub(1, Ordering::SeqCst);
            Poll::Ready(Some(value))
        })
    }
}
