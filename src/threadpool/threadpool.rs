use std::{
    backtrace, panic,
    sync::{atomic::AtomicBool, Arc, Barrier},
    thread,
};

use super::{queueops::QueueOperation, Func, QueueOrder, ThreadSafeQueue, WaitType};

pub struct ThreadPool {
    handles: Vec<thread::JoinHandle<()>>,
    count: usize,
    queue: ThreadSafeQueue<QueueOperation<Func>>,
    barrier: Arc<Barrier>,
    stop_flag: Arc<AtomicBool>,
    wait_type: WaitType,
}

impl Default for ThreadPool {
    fn default() -> Self {
        let queue = ThreadSafeQueue::new(QueueOrder::FirstOut);
        let count: usize;
        if let Ok(thread_count) = thread::available_parallelism() {
            count = thread_count.get();
        } else {
            count = 1;
        }
        let barrier = Arc::new(Barrier::new(count + 1));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let arc_flag = stop_flag.clone();
        let handles = (0..count)
            .map(|index| {
                let cloned_queue = queue.clone();
                let cloned_barrier = barrier.clone();
                let arc_flag = arc_flag.clone();
                start(index, cloned_queue, cloned_barrier, arc_flag)
            })
            .collect();
        ThreadPool {
            handles,
            queue,
            count,
            barrier,
            stop_flag,
            wait_type: WaitType::Cancel,
        }
    }
}

impl ThreadPool {
    pub fn new(count: usize, wait_type: WaitType) -> Option<Self> {
        if count == 0 {
            return None;
        }
        let queue = ThreadSafeQueue::new(QueueOrder::FirstOut);
        let barrier = Arc::new(Barrier::new(count + 1));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let arc_flag = stop_flag.clone();
        let handles = (0..count)
            .map(|index| {
                let cloned_queue = queue.clone();
                let cloned_barrier = barrier.clone();
                let arc_flag = arc_flag.clone();
                start(index, cloned_queue, cloned_barrier, arc_flag)
            })
            .collect();
        Some(ThreadPool {
            handles,
            queue,
            barrier,
            count,
            stop_flag,
            wait_type,
        })
    }

    pub fn new_with(count: usize, wait_type: WaitType) -> Option<Self> {
        if count == 0 {
            return None;
        }
        let queue = ThreadSafeQueue::new(QueueOrder::FirstOut);
        let barrier = Arc::new(Barrier::new(count + 1));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let handles = (0..count)
            .map(|index| {
                let cloned_queue = queue.clone();
                let cloned_barrier = barrier.clone();
                let arc_flag = stop_flag.clone();
                start(index, cloned_queue, cloned_barrier, arc_flag)
            })
            .collect();
        Some(ThreadPool {
            handles,
            queue,
            barrier,
            count,
            stop_flag,
            wait_type,
        })
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
        match self.wait_type {
            WaitType::Cancel => self.cancel_all(),
            WaitType::Wait => {
                self.wait_for_all();
                self.cancel_all();
            }
        }
        while let Some(handle) = self.handles.pop() {
            if !handle.is_finished() {
                let Ok(_) = handle.join() else {
                    continue;
                };
            }
        }
    }
}

fn start(
    index: usize,
    queue: ThreadSafeQueue<QueueOperation<Func>>,
    barrier: Arc<Barrier>,
    arc_flag: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    let name = format!("ThreadPool #{}", index);
    let handle = thread::Builder::new().name(name);
    handle
        .spawn(move || {
            panic::set_hook(Box::new(move |info: &panic::PanicInfo<'_>| {
                let msg = format!(
                    "{} panicked at location {} with {}\n{}",
                    thread::current().name().unwrap(),
                    info.location().unwrap(),
                    format_args!(
                        "{}",
                        info.to_string()
                            .split_terminator('\n')
                            .collect::<Vec<_>>()
                            .as_slice()[1]
                    ),
                    backtrace::Backtrace::capture()
                );
                eprintln!("{}", msg);
                _ = panic::take_hook();
                _ = panic::resume_unwind(Box::new(msg));
            }));
            for op in queue {
                match (op, arc_flag.load(std::sync::atomic::Ordering::Acquire)) {
                    (QueueOperation::Ready(work), false) => work(),
                    (QueueOperation::NotYet, false) => continue,
                    (QueueOperation::Wait, false) => _ = barrier.wait(),
                    _ => return,
                }
            }
        })
        .unwrap()
}
