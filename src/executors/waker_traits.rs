use std::{
    mem::ManuallyDrop,
    sync::Arc,
    task::{RawWaker, RawWakerVTable, Waker},
};

/// # Safety
/// All safe here
pub(crate) unsafe trait ViaRawPointer {
    type Target: ?Sized;

    fn into_raw(self) -> *mut Self::Target;

    unsafe fn from_raw(ptr: *mut Self::Target) -> Self;
}

pub(crate) trait WakeRef {
    fn wake_by_ref(&self);
}

pub(crate) trait Wake: WakeRef + Sized {
    #[inline]
    fn wake(self) {
        self.wake_by_ref()
    }
}

pub(crate) trait IntoWaker {
    const VTABLE: &'static RawWakerVTable;

    #[must_use]
    fn into_waker(self) -> Waker;
}

impl<T> IntoWaker for T
where
    T: Wake + Clone + 'static + ViaRawPointer,
    T::Target: Sized,
{
    const VTABLE: &'static RawWakerVTable = &RawWakerVTable::new(
        // clone
        |raw| {
            let raw = raw as *mut T::Target;

            let waker = ManuallyDrop::<T>::new(unsafe { ViaRawPointer::from_raw(raw) });
            let cloned: T = (*waker).clone();

            // We can't save the `into_raw` back into the raw waker, so we must
            // simply assert that the pointer has remained the same. This is
            // part of the ViaRawPointer safety contract, so we only check it
            // in debug builds.
            debug_assert_eq!(ManuallyDrop::into_inner(waker).into_raw(), raw);

            let cloned_raw = cloned.into_raw();
            let cloned_raw = cloned_raw as *const ();
            RawWaker::new(cloned_raw, T::VTABLE)
        },
        // wake by value
        |raw| {
            let raw = raw as *mut T::Target;
            let waker: T = unsafe { ViaRawPointer::from_raw(raw) };
            waker.wake();
        },
        // wake by ref
        |raw| {
            let raw = raw as *mut T::Target;
            let waker = ManuallyDrop::<T>::new(unsafe { ViaRawPointer::from_raw(raw) });
            waker.wake_by_ref();

            debug_assert_eq!(ManuallyDrop::into_inner(waker).into_raw(), raw);
        },
        // Drop
        |raw| {
            let raw = raw as *mut T::Target;
            let _waker: T = unsafe { ViaRawPointer::from_raw(raw) };
        },
    );

    fn into_waker(self) -> Waker {
        let raw = self.into_raw();
        let raw = raw as *const ();
        let raw_waker = RawWaker::new(raw, T::VTABLE);
        unsafe { Waker::from_raw(raw_waker) }
    }
}

unsafe impl<T: ?Sized> ViaRawPointer for Arc<T> {
    type Target = T;

    fn into_raw(self) -> *mut T {
        Arc::into_raw(self) as *mut T
    }

    unsafe fn from_raw(ptr: *mut T) -> Self {
        Arc::from_raw(ptr as *const T)
    }
}

impl<T: WakeRef + ?Sized> WakeRef for Arc<T> {
    #[inline]
    fn wake_by_ref(&self) {
        T::wake_by_ref(self.as_ref())
    }
}

impl<T: WakeRef + ?Sized> Wake for Arc<T> {}
