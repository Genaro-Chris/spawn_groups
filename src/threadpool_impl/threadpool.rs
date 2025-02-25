use std::{
    sync::{Arc, Barrier},
    thread::available_parallelism,
};

use crate::shared::priority_task::PrioritizedTask;

use super::{task_priority::TaskPriority, thread::UniqueThread};

pub(crate) struct ThreadPool {
    handles: Vec<UniqueThread>,
    index: usize,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        assert!(count > 0);
        ThreadPool {
            index: 0,
            handles: (1..=count).map(|_| UniqueThread::default()).collect(),
        }
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        let count: usize = available_parallelism()
            .unwrap_or(unsafe { std::num::NonZeroUsize::new_unchecked(1) })
            .get();

        ThreadPool {
            handles: (1..=count).map(|_| UniqueThread::default()).collect(),
            index: 0,
        }
    }
}

impl ThreadPool {
    pub(crate) fn submit(&mut self, task: PrioritizedTask<()>) {
        let old_index = self.index;
        self.index = (self.index + 1) % self.handles.len();
        self.handles[old_index].submit_task(task);
    }

    pub(crate) fn wait_for_all(&self) {
        let barrier = Arc::new(Barrier::new(self.handles.len() + 1));
        self.handles.iter().for_each(|channel| {
            channel.submit_task(PrioritizedTask::new_with(
                TaskPriority::Wait,
                barrier.clone(),
            ));
        });
        barrier.wait();
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
