//! Definition of containers of allocated memory.

use std::{
    alloc::{Layout, alloc, dealloc, handle_alloc_error},
    slice::{from_raw_parts, from_raw_parts_mut},
};

/// Marker trait for a blob of bytes.
pub trait Memory: AsRef<[u8]> + AsMut<[u8]> {}

// Safety: Raw pointer is only exposed via AsRef and AsMut.
unsafe impl Sync for Heap {}
unsafe impl Send for Heap {}

impl Memory for Heap {}

/// Memory allocated using the registered global allocator.
///
/// * If no custom allocator is registered, the default allocator from Rust std is used.
/// * Uses RAII pattern to free memory when heap memory goes out of scope.
pub struct Heap {
    len: usize,
    ptr: *mut u8,
    layout: Layout,
}

impl Heap {
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
    /// * `capacity` - Maximum number of items of type `T` the allocation should accommodate.
    pub(crate) fn alloc<T>(capacity: usize) -> Heap {
        // Size and alignment of the type held in ring buffer.
        let size = size_of::<T>();
        let align = align_of::<T>();

        // Layout of memory to allocate for the ring buffer.
        let len = size * capacity;
        let Ok(layout) = Layout::from_size_align(len, align) else {
            panic!("Trying to allocate more than isize::MAX");
        };

        // Allocate memory and track pointer to that memory.
        // We'll get a non-null pointer only if allocation was successful.
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            // This will never complete normally, i.e, if we reach this point,
            // no code after this will be executed. What exactly happens depends
            // on configuration of the binary using this crate.
            handle_alloc_error(layout);
        }

        // Return newly allocated memory.
        Self { len, ptr, layout }
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        // Safety
        // * Cannot initialize with invalid pointer and layout.
        unsafe {
            dealloc(self.ptr, self.layout);
        }
    }
}

impl AsRef<[u8]> for Heap {
    fn as_ref(&self) -> &[u8] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts(self.ptr, self.len) }
    }
}

impl AsMut<[u8]> for Heap {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { from_raw_parts_mut(self.ptr, self.len) }
    }
}
