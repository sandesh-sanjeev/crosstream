//! Definition of containers of allocated memory.

use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

/// Marker trait for a blob of bytes.
pub trait Memory<T>: AsRef<[T]> + AsMut<[T]> {}

// Safety: Raw pointer is only exposed via AsRef and AsMut.
unsafe impl<T> Sync for Heap<T> {}
unsafe impl<T> Send for Heap<T> {}

impl<T> Memory<T> for Heap<T> {}

/// Memory allocated using the registered global allocator.
///
/// * If no custom allocator is registered, the default allocator from Rust std is used.
/// * Uses RAII pattern to free memory when heap memory goes out of scope.
pub struct Heap<T> {
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
    pub(crate) fn alloc(cap: usize) -> Self {
        assert!(cap > 0, "zero length memory cannot be allocated");

        // Layout of memory to allocate for the ring buffer.
        let Ok(layout) = Layout::array::<T>(cap) else {
            panic!("Trying to allocate more than isize::MAX");
        };

        // Allocate memory and track pointer to that memory.
        // We'll get a non-null pointer only if allocation was successful.
        let ptr = match NonNull::new(unsafe { alloc(layout) as *mut T }) {
            Some(ptr) => ptr,
            None => handle_alloc_error(layout),
        };

        // Return newly allocated memory.
        Self { cap, ptr, layout }
    }
}

impl<T> Drop for Heap<T> {
    fn drop(&mut self) {
        // Safety
        // * Cannot initialize with invalid pointer and layout.
        unsafe {
            dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

impl<T> AsRef<[T]> for Heap<T> {
    fn as_ref(&self) -> &[T] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts(self.ptr.as_ptr(), self.cap) }
    }
}

impl<T> AsMut<[T]> for Heap<T> {
    fn as_mut(&mut self) -> &mut [T] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts_mut(self.ptr.as_ptr(), self.cap) }
    }
}
