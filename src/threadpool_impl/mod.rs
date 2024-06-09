mod channel;
mod index;
mod thread;
mod threadpool;

pub(crate) type Func = dyn FnOnce() + Send;

pub(crate) use channel::Channel;
pub(crate) use threadpool::ThreadPool;
