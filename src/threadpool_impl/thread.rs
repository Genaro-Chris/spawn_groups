use std::thread;

pub(crate) struct UniqueThread {
    handle: thread::JoinHandle<()>,
}

impl UniqueThread {
    pub(crate) fn new<Task: FnOnce() + Send + 'static>(name: String, task: Task) -> Self {
        let handle = thread::Builder::new()
            .name(name)
            .spawn(move || {
                task();
            })
            .unwrap();
        UniqueThread { handle }
    }
}

impl UniqueThread {
    pub(crate) fn join(self) {
        _ = self.handle.join();
    }
}
