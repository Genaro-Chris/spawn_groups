use std::thread::spawn;

use super::{Channel, Func};

pub(crate) struct UniqueThread {
    channel: Channel<Box<Func>>,
}

impl UniqueThread {
    pub(crate) fn new() -> Self {
        let channel: Channel<Box<dyn FnOnce() + Send>> = Channel::new();
        let chan: Channel<Box<dyn FnOnce() + Send>> = channel.clone();
        spawn(move || {
            while let Some(ops) = chan.dequeue() {
                ops()
            }
        });
        UniqueThread { channel }
    }
}

impl UniqueThread {
    pub(crate) fn join(self) {
        self.channel.close();
        self.channel.clear();
    }

    pub(crate) fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + Send + 'static,
    {
        self.channel.enqueue(Box::new(task));
    }
}
