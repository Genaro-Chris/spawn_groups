use std::{
    backtrace, panic,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    thread,
};

use super::{queueops::QueueOperation, thread::UniqueThread, Func, ThreadSafeQueue};

pub struct ThreadPool {
    handles: Vec<UniqueThread>,
    count: usize,
    queue: ThreadSafeQueue<QueueOperation<Func>>,
    barrier: Arc<Barrier>,
    stop_flag: Arc<AtomicBool>,
}

impl Default for ThreadPool {
    fn default() -> Self {
        panic_hook();
        let queue = ThreadSafeQueue::new();
        let count: usize;
        if let Ok(thread_count) = thread::available_parallelism() {
            count = thread_count.get();
        } else {
            count = 1;
        }
        let barrier = Arc::new(Barrier::new(count + 1));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let handles = (0..count)
            .map(|index| start(index, queue.clone(), barrier.clone(), stop_flag.clone()))
            .collect();
        ThreadPool {
            handles,
            queue,
            count,
            barrier,
            stop_flag,
        }
    }
}

impl ThreadPool {
    pub fn submit<Task>(&self, task: Task)
    where
        Task: FnOnce() + 'static + Send,
    {
        self.queue.enqueue(QueueOperation::Ready(Box::new(task)));
    }
}

impl ThreadPool {
    pub fn wait_for_all(&self) {
        for _ in 0..self.count {
            self.queue.enqueue(QueueOperation::Wait);
        }
        self.barrier.wait();
    }
}

impl ThreadPool {
    fn cancel_all(&self) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::Release)
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        _ = panic::take_hook();
        self.cancel_all();
        while let Some(handle) = self.handles.pop() {
            handle.join();
        }
    }
}

fn start(
    index: usize,
    queue: ThreadSafeQueue<QueueOperation<Func>>,
    barrier: Arc<Barrier>,
    stop_flag: Arc<AtomicBool>,
) -> UniqueThread {
    UniqueThread::new(format!("ThreadPool #{}", index), move || {
        for op in queue {
            match (op, stop_flag.load(Ordering::Acquire)) {
                (QueueOperation::NotYet, false) => continue,
                (QueueOperation::Ready(work), false) => {
                    work();
                }
                (QueueOperation::Wait, false) => _ = barrier.wait(),
                _ => {
                    return;
                }
            }
        }
    })
}

fn panic_hook() {
    panic::set_hook(Box::new(move |info: &panic::PanicInfo<'_>| {
        let msg = format!(
            "{} panicked at location {} with {} \nBacktrace:\n{}",
            thread::current().name().unwrap(),
            info.location().unwrap(),
            info.to_string().split('\n').collect::<Vec<_>>()[1],
            backtrace::Backtrace::capture()
        );
        eprintln!("{}", msg);
        _ = panic::take_hook();
    }));
}
