use std::{
    collections::VecDeque,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

use async_lock::Mutex;
use futures_lite::Stream;

pub struct AsyncStream<ItemType> {
    inner: Arc<Inner<ItemType>>,
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn insert_item(&self, value: ItemType) {
        let mut inner_lock = self.inner.inner_lock.lock().await;
        inner_lock.buffer.push_back(value);
        // check if any waker was registered
        let Some(waker) = inner_lock.wakers.take() else {
            return;
        };

        drop(inner_lock);

        // wakeup the waker
        waker.wake();
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) async fn buffer_count(&self) -> usize {
        self.inner.inner_lock.lock().await.buffer.len()
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn increment(&self) {
        self.inner.item_count.fetch_add(1, Ordering::Relaxed);
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub async fn first(&self) -> Option<ItemType> {
        let mut inner_lock = self.inner.inner_lock.lock().await;
        if inner_lock.buffer.is_empty() || self.item_count() == 0 {
            return None;
        }

        let value = inner_lock.buffer.pop_front()?;
        self.inner.item_count.fetch_sub(1, Ordering::Relaxed);
        Some(value)
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn item_count(&self) -> usize {
        self.inner.item_count.load(Ordering::Acquire)
    }
}

impl<ItemType> Clone for AsyncStream<ItemType> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    pub(crate) fn new() -> Self {
        AsyncStream {
            inner: Arc::new(Inner::new()),
        }
    }
}

struct Inner<ItemType> {
    inner_lock: Mutex<InnerState<ItemType>>,
    item_count: AtomicUsize,
}

impl<ItemType> Inner<ItemType> {
    fn new() -> Self {
        Self {
            inner_lock: Mutex::new(InnerState::new()),
            item_count: AtomicUsize::new(0),
        }
    }
}

enum Stages<T> {
    Empty,
    Wait,
    Ready(T),
}

struct InnerState<ItemType> {
    buffer: VecDeque<ItemType>,
    wakers: Option<Waker>,
}

impl<T> InnerState<T> {
    fn new() -> InnerState<T> {
        Self {
            buffer: VecDeque::new(),
            wakers: None,
        }
    }
}

impl<ItemType> AsyncStream<ItemType> {
    fn poll_item(&self, cx: &mut Context<'_>) -> Poll<Stages<ItemType>> {
        if self.item_count() == 0 {
            return Poll::Ready(Stages::Empty);
        }
        let waker = cx.waker().clone();
        let mut future = async move {
            let mut inner_lock = self.inner.inner_lock.lock().await;
            if self.item_count() == 0 && inner_lock.buffer.is_empty() {
                return Stages::Empty;
            }
            let Some(value) = inner_lock.buffer.pop_front() else {
                // register the waker so we can called it later
                inner_lock.wakers.replace(waker);
                return Stages::Wait;
            };

            self.inner.item_count.fetch_sub(1, Ordering::Relaxed);
            Stages::Ready(value)
        };
        unsafe { Pin::new_unchecked(&mut future) }.poll(cx)
    }
}

impl<ItemType> Stream for AsyncStream<ItemType> {
    type Item = ItemType;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.poll_item(cx) {
            Poll::Pending => {
                // This means the lock has not been acquired yet
                // so immediately wake up this waker
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(stage) => match stage {
                Stages::Empty => Poll::Ready(None),
                Stages::Wait => Poll::Pending,
                Stages::Ready(value) => Poll::Ready(Some(value)),
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.item_count()))
    }
}
