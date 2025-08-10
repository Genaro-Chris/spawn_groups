use std::{
    panic::catch_unwind,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar,
    },
    task::Waker,
    thread,
};

use crate::shared::{
    block_on, mutex::StdMutex, priority_task::PrioritizedTask, Suspender, TaskOrBarrier, WAKER_PAIR,
};

use super::PriorityQueue;

pub(crate) struct EventLoop {
    inner: Arc<Inner>,
}

impl EventLoop {
    pub(crate) fn new(index: usize) -> Self {
        let inner = Arc::new(Inner::new());
        let inner_clone = inner.clone();
        _ = thread::Builder::new()
            .name(format!("Eventloop #{index}"))
            .spawn(move || {
                WAKER_PAIR.with(|pair| {
                    inner_clone.start(pair);
                });
            });
        Self { inner }
    }

    pub(crate) fn clear(&self) {
        self.inner.clear();
    }

    pub(crate) fn end(&self) {
        self.inner.end();
    }

    pub(crate) fn submit_task(&self, value: PrioritizedTask<()>) {
        self.inner.enqueue(value);
    }
}

struct Inner {
    m_mutex: StdMutex<PriorityQueue<PrioritizedTask<()>>>,
    m_condvar: Condvar,
    m_running: AtomicBool,
}

impl Inner {
    fn new() -> Self {
        Self {
            m_mutex: StdMutex::new(PriorityQueue::new(
                |lhs: &PrioritizedTask<()>, rhs: &PrioritizedTask<()>| {
                    lhs.priority() > rhs.priority()
                },
            )),
            m_condvar: Condvar::new(),
            m_running: AtomicBool::new(true),
        }
    }

    fn enqueue(&self, value: PrioritizedTask<()>) {
        self.m_mutex.lock().push(value);
        self.m_condvar.notify_one();
    }

    fn clear(&self) {
        self.m_mutex.lock().clear();
    }

    fn end(&self) {
        self.m_running.store(false, Ordering::Release);
        self.m_mutex.lock().clear();
        self.m_condvar.notify_all();
    }

    fn start(&self, waker_pair: &(Arc<Suspender>, Waker)) {
        let mut read_buffer =
            PriorityQueue::new(|lhs: &PrioritizedTask<()>, rhs: &PrioritizedTask<()>| {
                lhs.priority() > rhs.priority()
            });

        while self.m_running.load(Ordering::Acquire) {
            {
                let mut lock = self.m_mutex.lock();
                while lock.is_empty() && self.m_running.load(Ordering::Acquire) {
                    lock = self.m_condvar.wait(lock).unwrap();
                }
                std::mem::swap(&mut *lock, &mut read_buffer);
            }

            while let Some(task) = read_buffer.pop() {
                match task.task {
                    TaskOrBarrier::Task(task) => {
                        _ = catch_unwind(|| block_on(task, waker_pair));
                    }
                    TaskOrBarrier::Barrier(barrier) => {
                        barrier.wait();
                    }
                }
            }

            read_buffer.clear();
        }
    }
}
