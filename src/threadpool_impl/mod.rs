mod channel;
mod threadpool;

pub(crate) type Func = dyn FnOnce() + Send;

pub(crate) use channel::Channel;
pub(crate) use threadpool::ThreadPool;
