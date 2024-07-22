use std::{
    backtrace, panic,
    sync::atomic::{AtomicUsize, Ordering},
    thread::spawn,
};

use super::{waitgroup::WaitGroup, Channel, Func};

pub(crate) struct ThreadPool {
    task_channels: Vec<Channel<Box<Func>>>,
    index: AtomicUsize,
    wait_group: WaitGroup,
}

impl ThreadPool {
    pub(crate) fn new(count: usize) -> Self {
        let mut count = count;
        if count < 1 {
            count = 1;
        }
        ThreadPool {
            task_channels: (1..=count)
                .map(|_| {
                    let channel: Channel<Box<Func>> = Channel::new();
                    let chan = channel.clone();
                    spawn(move || {
                        panic_hook();
                        for ops in channel {
                            ops();
                        }
                    });
                    chan
                })
                .collect(),
            index: AtomicUsize::new(0),
            wait_group: WaitGroup::new(),
        }
    }
}

impl ThreadPool {
    fn current_index(&self) -> usize {
        self.index.swap(
            (self.index.load(Ordering::Relaxed) + 1) % self.task_channels.len(),
            Ordering::SeqCst,
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
        _ = panic::take_hook();
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
