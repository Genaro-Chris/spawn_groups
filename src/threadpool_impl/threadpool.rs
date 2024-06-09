use std::sync::{Arc, Barrier};

use super::{index::Indexer, thread::UniqueThread};

pub struct ThreadPool {
    handles: Vec<UniqueThread>,
    indexer: Indexer,
    barrier: Arc<Barrier>,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        let mut handles = vec![];
        handles.reserve(count);
        for _ in 1..=count {
            handles.push(UniqueThread::new());
        }
        ThreadPool {
            handles,
            indexer: Indexer::new(count),
            barrier: Arc::new(Barrier::new(count + 1)),
        }
    }
}

impl ThreadPool {
    pub fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + 'static + Send,
    {
        self.handles[self.indexer.next()].submit(task);
    }

    pub fn clear(&self) {
        self.handles.iter().for_each(|handles| handles.clear());
    }
}

impl ThreadPool {
    pub fn wait_for_all(&self) {
        self.handles.iter().for_each(|handle| {
            let barrier = self.barrier.clone();
            handle.submit(move || {
                barrier.wait();
            });
        });
        self.barrier.wait();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        while let Some(handle) = self.handles.pop() {
            handle.join()
        }
    }
}
