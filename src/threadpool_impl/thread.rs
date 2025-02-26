use std::{
    panic::catch_unwind,
    sync::Arc,
    task::{Context, Poll},
    thread::spawn,
};

use crate::shared::{priority_task::PrioritizedTask, Suspender, Task, TaskOrBarrier, WAKER_PAIR};

use super::channel::Channel;

pub(crate) struct UniqueThread {
    task_channel: Arc<Channel<PrioritizedTask<()>>>,
}

impl UniqueThread {
    pub(crate) fn submit_task(&self, task: PrioritizedTask<()>) {
        self.task_channel.enqueue(task);
    }

    pub(crate) fn clear(&self) {
        self.task_channel.clear()
    }

    pub(crate) fn end(&self) {
        self.task_channel.end()
    }
}

impl Default for UniqueThread {
    fn default() -> Self {
        let task_channel: Arc<Channel<PrioritizedTask<()>>> = Arc::new(Channel::new());
        let chan = task_channel.clone();
        spawn(move || {
            WAKER_PAIR.with(|waker_pair| loop {
                let Some(task) = chan.dequeue() else { return };
                let mut context = Context::from_waker(&waker_pair.1);
                match task.task {
                    TaskOrBarrier::Task(mut task) => {
                        drop(catch_unwind(move || {
                            poll_task(&mut task, &waker_pair.0, &mut context)
                        }));
                    }
                    TaskOrBarrier::Barrier(barrier) => {
                        barrier.wait();
                    }
                }
            });
        });
        Self { task_channel }
    }
}

fn poll_task(task: &mut Task<()>, suspender: &Arc<Suspender>, context: &mut Context<'_>) {
    loop {
        match task.poll_task(context) {
            Poll::Ready(()) => return,
            Poll::Pending => {
                suspender.suspend();
            }
        }
    }
}
