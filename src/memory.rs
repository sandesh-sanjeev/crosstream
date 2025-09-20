//! Definition of containers of allocated memory.

use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

// Safety: Raw pointer is only exposed via AsRef and AsMut.
unsafe impl<T> Sync for Heap<T> {}
unsafe impl<T> Send for Heap<T> {}

/// Memory allocated using the registered global allocator.
///
/// * If no custom allocator is registered, the default allocator from Rust std is used.
/// * Uses RAII pattern to free memory when heap memory goes out of scope.
pub(crate) struct Heap<T> {
    cap: usize,
    layout: Layout,
    ptr: NonNull<T>,
}

impl<T> Heap<T> {
    /// Allocate some memory on heap.
    ///
    /// Memory will be aligned to the alignment of the generic type.
    ///
    /// Uses the global allocator to allocate memory. Maybe in the future we will
    /// provide the ability to allocate memory using a custom allocator. We'll
    /// consider that once `Allocator` trait is stabilized in Rust.
    ///
    /// # Arguments
    ///
    /// * `cap` - Maximum number of items this allocation was accommodate.
    #[track_caller]
    pub(crate) fn alloc(cap: usize) -> Self {
        assert!(cap > 0, "zero length memory cannot be allocated");

        // Layout of memory to allocate for the ring buffer.
        let layout = Layout::array::<T>(cap)
            .expect("Trying to allocate more than isize::MAX worth of memory");

        // Allocate memory and track pointer to that memory.
        // We'll get a non-null pointer only if allocation was successful.
        // Safety: We just made sure layout is correct.
        let ptr = NonNull::new(unsafe { alloc(layout) as *mut T })
            .unwrap_or_else(|| handle_alloc_error(layout));

        // Return newly allocated memory.
        Self { cap, ptr, layout }
    }
}

impl<T> Drop for Heap<T> {
    #[inline]
    fn drop(&mut self) {
        // Safety
        // * Cannot initialize with invalid pointer and layout.
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

impl<T> Deref for Heap<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.cap) }
    }
}

impl<T> DerefMut for Heap<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.cap) }
    }
}
