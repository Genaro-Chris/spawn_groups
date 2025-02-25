use std::{
    mem::ManuallyDrop,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

use super::suspender::Suspender;

pub(crate) fn waker_helper(suspender: Arc<Suspender>) -> Waker {
    let raw: *const () = Arc::into_raw(suspender) as *const ();
    unsafe {
        Waker::from_raw(RawWaker::new(
            raw,
            &RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker),
        ))
    }
}

// clones the waker
pub(crate) unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
    let ptr = ptr as *mut Suspender;
    let waker = ManuallyDrop::new(Arc::from_raw(ptr));
    let cloned = (*waker).clone();

    debug_assert_eq!(Arc::into_raw(ManuallyDrop::into_inner(waker)), ptr);

    let cloned_ptr = Arc::into_raw(cloned) as *const ();

    RawWaker::new(
        cloned_ptr,
        &RawWakerVTable::new(clone_waker, wake, wake_by_ref, drop_waker),
    )
}

// wakes up by consuming it
pub(crate) unsafe fn wake(ptr: *const ()) {
    let ptr = ptr as *mut Suspender;
    let waker = Arc::from_raw(ptr);
    waker.resume();
}

// wakes up by reference
pub(crate) unsafe fn wake_by_ref(ptr: *const ()) {
    let ptr = ptr as *mut Suspender;

    let waker = ManuallyDrop::new(Arc::from_raw(ptr));
    waker.resume();

    debug_assert_eq!(Arc::into_raw(ManuallyDrop::into_inner(waker)), ptr);
}

// drops the waker
pub(crate) unsafe fn drop_waker(ptr: *const ()) {
    let ptr = ptr as *mut Suspender;
    drop(Arc::from_raw(ptr));
}
