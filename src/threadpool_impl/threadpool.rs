use std::{
    sync::{Arc, Barrier},
    thread::available_parallelism,
};

use crate::shared::priority_task::PrioritizedTask;

use super::eventloop::EventLoop;

pub(crate) struct ThreadPool {
    handles: Vec<EventLoop>,
    barrier: Arc<Barrier>,
    index: usize,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        assert!(count > 0);
        ThreadPool {
            index: 0,
            barrier: Arc::new(Barrier::new(count + 1)),
            handles: (1..=count).map(EventLoop::new).collect(),
        }
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        let count: usize = available_parallelism()
            .unwrap_or(std::num::NonZeroUsize::new(1).unwrap())
            .get();

        ThreadPool::new(count)
    }
}

impl ThreadPool {
    pub(crate) fn submit(&mut self, task: PrioritizedTask<()>) {
        let old_index = self.index;
        self.index = (self.index + 1) % self.handles.len();
        self.handles[old_index].submit_task(task);
    }

    pub(crate) fn wait_for_all(&self) {
        self.handles.iter().for_each(|channel| {
            channel.submit_task(PrioritizedTask::new_with(self.barrier.clone()));
        });
        self.barrier.wait();
    }
}

impl ThreadPool {
    pub(crate) fn end(&self) {
        self.handles.iter().for_each(|channel| channel.end());
    }
}

impl ThreadPool {
    pub(crate) fn clear(&self) {
        self.handles.iter().for_each(|channel| channel.clear());
    }
}
