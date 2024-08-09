use std::{
    sync::atomic::{AtomicUsize, Ordering},
    thread::spawn,
};

use super::{waitgroup::WaitGroup, Channel, Func};

pub(crate) struct ThreadPool {
    index: AtomicUsize,
    task_channels: Vec<Channel<Box<Func>>>,
    wait_group: WaitGroup,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        let mut task_channels = Vec::with_capacity(count);
        for _ in 1..=count {
            let channel: Channel<Box<Func>> = Channel::new();
            let chan = channel.clone();
            spawn(move || {
                while let Some(ops) = channel.dequeue() {
                    ops();
                }
            });
            task_channels.push(chan);
        }
        ThreadPool {
            index: AtomicUsize::new(0),
            task_channels,
            wait_group: WaitGroup::new(),
        }
    }
}

impl ThreadPool {
    fn current_index(&self) -> usize {
        self.index.swap(
            (self.index.load(Ordering::Relaxed) + 1) % self.task_channels.len(),
            Ordering::Relaxed,
        )
    }

    pub(crate) fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + 'static + Send,
    {
        self.task_channels[self.current_index()].enqueue(Box::new(task));
    }

    pub(crate) fn wait_for_all(&self) {
        self.task_channels.iter().for_each(|channel| {
            let wait_group = self.wait_group.clone();
            wait_group.enter();
            channel.enqueue(Box::new(move || {
                wait_group.leave();
            }));
        });
        self.wait_group.wait();
    }
}

impl ThreadPool {
    pub(crate) fn end(&self) {
        self.task_channels.iter().for_each(|channel| {
            channel.close();
            channel.clear();
        });
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.end();
    }
}

impl ThreadPool {
    pub(crate) fn clear(&self) {
        self.task_channels
            .iter()
            .for_each(|channel| channel.clear());
    }
}
