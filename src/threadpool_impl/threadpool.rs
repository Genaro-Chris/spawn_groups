use std::{
    backtrace, panic, sync::{Arc, Barrier}, thread::spawn
};

use super::{Channel, Func};

pub struct ThreadPool {
    task_channel: Channel<Box<Func>>,
    barrier: Arc<Barrier>,
    count: usize,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        let task_channel: Channel<Box<Func>> = Channel::new();
        for _ in 1..=count {
            let channel: Channel<Box<Func>> = task_channel.clone();
            spawn(move || {
                panic_hook();
                while let Some(ops) = channel.dequeue() {
                    ops()
                }
            });
        }
        ThreadPool {
            task_channel,
            count,
            barrier: Arc::new(Barrier::new(count + 1)),
        }
    }
}

impl ThreadPool {
    pub fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + 'static + Send,
    {
        self.task_channel.enqueue(Box::new(task));
    }
}

impl ThreadPool {
    pub fn wait_for_all(&self) {
        (1..=self.count).for_each(|_| {
            let barrier: Arc<Barrier> = self.barrier.clone();
            self.task_channel.enqueue(Box::new(move || {
                barrier.wait();
            }));
        });
        self.barrier.wait();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        _ = panic::take_hook();
        self.task_channel.close();
        self.clear()
    }
}

impl ThreadPool {
    pub fn clear(&self) {
        self.task_channel.clear();
    }
}

fn panic_hook() {
    panic::set_hook(Box::new(move |info: &panic::PanicInfo<'_>| {
        let msg = format!(
            "Threadpool panicked at location {} with {} \nBacktrace:\n{}",
            info.location().unwrap(),
            info.to_string().split('\n').collect::<Vec<_>>()[1],
            backtrace::Backtrace::capture()
        );
        eprintln!("{}", msg);
        _ = panic::take_hook();
    }));
}
