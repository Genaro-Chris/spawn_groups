use std::{
    mem,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

// Waker implementation
struct WakerHelper<F>(F);

pub(crate) fn waker_helper<F: Fn()>(f: F) -> Waker {
    let raw: *const () = Arc::into_raw(Arc::new(f)) as *const ();
    let vtable: &RawWakerVTable = &WakerHelper::<F>::VTABLE;
    unsafe { Waker::from_raw(RawWaker::new(raw, vtable)) }
}

impl<F: Fn()> WakerHelper<F> {
    // A virtual function table (vtable) that specifies the behavior of a RawWaker
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );

    // clones the waker
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let arc: mem::ManuallyDrop<Arc<F>> = mem::ManuallyDrop::new(Arc::from_raw(ptr as *const F));
        _ = arc.clone();
        RawWaker::new(ptr, &Self::VTABLE)
    }

    // wakes up by consuming it
    unsafe fn wake(ptr: *const ()) {
        let arc: Arc<F> = Arc::from_raw(ptr as *const F);
        (arc)();
    }

    // wakes up by reference
    unsafe fn wake_by_ref(ptr: *const ()) {
        let arc: mem::ManuallyDrop<Arc<F>> = mem::ManuallyDrop::new(Arc::from_raw(ptr as *const F));
        (arc)();
    }

    // drops the waker
    unsafe fn drop_waker(ptr: *const ()) {
        drop(Arc::from_raw(ptr as *const F))
    }
}
