pub(crate) enum QueueOperation<T: Send + ?Sized> {
    Ready(Box<T>),
    NotYet,
    Wait,
}