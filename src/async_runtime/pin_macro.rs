/// Pins the variable implementing the ``std::future::Future`` trait onto the stack
#[macro_export]
macro_rules! pin_future {
    ($x:ident) => {
        let mut $x = $x;
        let mut $x = unsafe { std::pin::Pin::new_unchecked(&mut $x) };
    };
}
