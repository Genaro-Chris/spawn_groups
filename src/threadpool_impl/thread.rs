use std::thread::{spawn, JoinHandle};

use super::{Channel, Func};

pub(crate) struct UniqueThread {
    channel: Channel<Box<Func>>,
    handle: JoinHandle<()>,
}

impl UniqueThread {
    pub(crate) fn new() -> Self {
        let channel: Channel<Box<dyn FnOnce() + Send>> = Channel::new();
        let chan = channel.clone();
        let handle = spawn(move || {
            while let Some(ops) = chan.dequeue() {
                ops()
            }
        });
        UniqueThread { channel, handle }
    }
}

impl UniqueThread {
    pub(crate) fn join(self) {
        self.channel.close();
        self.channel.clear();
        _ = self.handle.join().unwrap();
    }

    pub(crate) fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + Send + 'static,
    {
        self.channel.enqueue(Box::new(task));
    }

    pub(crate) fn clear(&self) {
        self.channel.clear();
    }
}
