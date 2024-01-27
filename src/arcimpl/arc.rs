use std::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error, Layout},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// Automatic reference counted container with inner mutability unlike the `std::sync::Arc`
pub struct CustomArc<T> {
    count: Arc<AtomicUsize>,
    unsafe_ptr: *mut T,
    phantom: PhantomData<T>,
}

impl<T> CustomArc<T> {
    pub(crate) fn add_ref(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> *mut T {
        self.unsafe_ptr
    }

    pub fn new(value: T) -> CustomArc<T> {
        Self {
            count: Arc::new(AtomicUsize::new(1)),
            unsafe_ptr: unsafe {
                let layout = Layout::new::<T>();
                let unsafe_ptr = alloc(layout) as *mut T;
                if unsafe_ptr.is_null() {
                    handle_alloc_error(layout);
                }
                unsafe_ptr.write(value);
                unsafe_ptr
            },
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for CustomArc<T> {
    fn drop(&mut self) {
        if self.count.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe {
                let ptr = NonNull::new_unchecked(self.unsafe_ptr);
                dealloc(ptr.as_ptr() as *mut u8, Layout::new::<T>());
            }
        }
    }
}

impl<T> Deref for CustomArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.unsafe_ptr }
    }
}

impl<T> DerefMut for CustomArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.unsafe_ptr }
    }
}

unsafe impl<T> Send for CustomArc<T> {}

unsafe impl<T> Sync for CustomArc<T> {}

impl<T: Default> Default for CustomArc<T> {
    fn default() -> Self {
        Self {
            count: Arc::new(AtomicUsize::new(1)),
            unsafe_ptr: unsafe {
                let layout = Layout::new::<T>();
                let unsafe_ptr = alloc_zeroed(layout) as *mut T;
                if unsafe_ptr.is_null() {
                    handle_alloc_error(layout);
                }
                unsafe_ptr.write(T::default());
                unsafe_ptr
            },
            phantom: PhantomData,
        }
    }
}

impl<T> Clone for CustomArc<T> {
    fn clone(&self) -> Self {
        self.add_ref();
        Self {
            count: self.count.clone(),
            unsafe_ptr: self.unsafe_ptr,
            phantom: PhantomData,
        }
    }
}
