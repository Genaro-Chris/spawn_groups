mod iteratorimpl;
mod queue;
mod queueops;
mod threadpool;
mod thread;

pub(crate) type Func = dyn FnOnce() + Send;

pub(crate) use queue::ThreadSafeQueue;
pub(crate) use queueops::QueueOperation;
pub(crate) use threadpool::ThreadPool;
