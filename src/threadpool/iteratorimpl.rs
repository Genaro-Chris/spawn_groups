use super::{QueueOperation, ThreadSafeQueue, Func};

impl Iterator for ThreadSafeQueue<QueueOperation<Func>> {
    type Item = QueueOperation<Func>;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(value) = self.dequeue() else {
            return Some(QueueOperation::NotYet);
        };
        Some(value)
    }
}
