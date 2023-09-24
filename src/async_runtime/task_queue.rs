use futures_lite::StreamExt;
use super::{stream::AsyncStream, task::Task};

#[derive(Clone)]
pub struct TaskQueue {
    pub(crate) stream: AsyncStream<Task>,
}

impl TaskQueue {
    pub(crate) fn new() -> Self {
        Self {
            stream: AsyncStream::new(),
        }
    }

    pub fn push(&mut self, task: Task) {
        self.stream.insert_item(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        futures_lite::future::block_on(async move { self.stream.next().await })
    }
}
