use std::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error, Layout},
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// Automatic reference counted container with inner mutability unlike the `std::sync::Arc`
pub struct ARC<T> {
    count: Arc<AtomicUsize>,
    ptr: *mut T,
}

impl<T> ARC<T> {
    pub(crate) fn add_ref(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> *mut T {
        self.ptr
    }

    pub fn new(value: T) -> ARC<T> {
        let ptr = unsafe {
            let layout = Layout::new::<T>();
            let ptr = alloc(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            *(ptr as *mut T) = value;
            ptr as *mut T
        };
        Self {
            count: Arc::new(AtomicUsize::new(1)),
            ptr,
        }
    }
}

impl<T> Drop for ARC<T> {
    fn drop(&mut self) {
        if self.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe { dealloc(self.ptr as *mut u8, Layout::new::<T>()) }
        }
    }
}

impl<T> Deref for ARC<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for ARC<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

unsafe impl<T> Send for ARC<T> {}

unsafe impl<T> Sync for ARC<T> {}

impl<T: Default> Default for ARC<T> {
    fn default() -> Self {
        let layout = Layout::new::<T>();
        let ptr = unsafe {
            let ptr = alloc_zeroed(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            ptr as *mut T
        };
        Self {
            count: Arc::new(AtomicUsize::new(1)),
            ptr,
        }
    }
}

impl<T> Clone for ARC<T> {
    fn clone(&self) -> Self {
        self.add_ref();
        Self {
            count: self.count.clone(),
            ptr: self.ptr,
        }
    }
}
