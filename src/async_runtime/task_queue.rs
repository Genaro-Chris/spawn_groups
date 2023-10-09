use super::{stream::AsyncStream, task::Task};
use crate::executors::block_on;
use futures_lite::StreamExt;

#[derive(Clone, Default)]
pub struct TaskQueue {
    pub(crate) stream: AsyncStream<Task>,
}

impl TaskQueue {
    pub(crate) fn push(&mut self, task: Task) {
        self.stream.insert_item(task);
    }

    pub(crate) fn pop(&mut self) -> Option<Task> {
        block_on(async move { self.stream.next().await })
    }
}
