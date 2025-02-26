use std::sync::{Arc, Barrier};

use super::task::Task;

// Naming is hard guys
pub(crate) enum TaskOrBarrier<T> {
    Task(Task<T>),
    Barrier(Arc<Barrier>),
}
