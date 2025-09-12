//! Definition of storage engine that backs a segment.

use crate::Record;
use core::slice;
use memmap2::{MmapMut, MmapOptions};
use std::{
    alloc::{self, Layout, handle_alloc_error},
    marker::PhantomData,
};

/// Type alias for [`MemStorage`] backed by [`OnHeap`] memory.
pub type OnHeapStorage<T> = MemStorage<T, OnHeap>;

/// Type alias for [`MemStorage`] backed by [`OffHeap`] memory.
pub type OffHeapStorage<T> = MemStorage<T, OffHeap>;

/// Storage engine that holds a contiguous sequence of records.
///
/// # Internal
///
/// For the most part this is a trait that is internal to this crate. It's only
/// exposed externally because this is part of type signature for types that are
/// exposed externally. You cannot access this storage from ring buffer instance.
/// Although you can implement this trait, it's useless cause you cannot use it
/// with this ring buffer.
pub trait Storage {
    /// Associated type for records stored.
    type Record;

    /// Total number of records that Storage can accommodate.
    fn capacity(&self) -> usize;

    /// Number of records currently in storage.
    fn length(&self) -> usize;

    /// Number of records that can be appended to storage without overflow.
    fn remaining(&self) -> usize;

    /// Trim first len records from storage.
    ///
    /// # Invariants
    ///
    /// * len <= self.len()
    ///
    /// # Arguments
    ///
    /// * `len` - Number of records to trim.
    fn trim(&mut self, len: usize);

    /// Append some records into storage.
    ///
    /// # Invariants
    ///
    /// * records.len() should be <= self.remaining()
    ///
    /// # Arguments
    ///
    /// * `records` - Records to append into storage.
    fn extend(&mut self, records: &[Self::Record]);

    /// Clear all records from storage.
    fn clear(&mut self);

    /// Return reference to all records in storage.
    fn records(&self) -> &[Self::Record];
}

/// Storage engine that uses [`Vec`] for memory.
///
/// This is nothing but a thin wrapper around [`Vec`] that implements
/// the [`Storage`] trait.
#[derive(Debug)]
pub struct VecStorage<T>(Vec<T>);

impl<T: Record + Copy> VecStorage<T> {
    /// Create a new instance of [`Vec`] backed [`Storage`] engine.
    ///
    /// # Panics
    ///
    /// Panics if requested capacity could not be allocated.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum capacity of this storage engine.
    pub(crate) fn new(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }
}

impl<T: Copy> Storage for VecStorage<T> {
    type Record = T;

    fn capacity(&self) -> usize {
        self.0.capacity()
    }

    fn length(&self) -> usize {
        self.0.len()
    }

    fn remaining(&self) -> usize {
        self.capacity() - self.length()
    }

    fn trim(&mut self, len: usize) {
        self.0.drain(..len);
    }

    fn extend(&mut self, records: &[T]) {
        self.0.extend(records);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn records(&self) -> &[T] {
        &self.0
    }
}

/// Storage engine that uses raw (byte addressable) memory to hold records.
///
/// # Safety
///
/// We mostly treat records and storage as just a blob of bytes. Unsafe allows
/// us to use pure pointer arithmetic + memcpy/memove to move records into and
/// out of storage without any bounds checking. For this to all work correctly
/// all invariants specified in [`Storage`] must be held true.
///
/// In theory we can also get rid of all the overflow checks assuming invariants
/// hold true. As a future TODO, check if it's worth it.
#[derive(Debug)]
pub struct MemStorage<T, M> {
    mem: M,
    length: usize,
    capacity: usize,
    phantom: PhantomData<T>,
}

impl<T: Record> OffHeapStorage<T> {
    /// Create a new instance of [`Storage`] engine backed by off-heap memory.
    ///
    /// # Panics
    ///
    /// Panics if requested capacity could not be allocated.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum capacity of this storage engine.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            mem: OffHeap::alloc(capacity * T::size()),
            length: 0,
            capacity,
            phantom: PhantomData,
        }
    }
}

impl<T: Record> OnHeapStorage<T> {
    /// Create a new instance of [`Storage`] engine backed by on-heap memory.
    ///
    /// # Panics
    ///
    /// * Requested capacity could not be allocated.
    /// * capacity == 0 or type is zero sized type.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum capacity of this storage engine.
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            mem: OnHeap::alloc(capacity * T::size()),
            length: 0,
            capacity,
            phantom: PhantomData,
        }
    }
}

impl<T: Record, M> Storage for MemStorage<T, M>
where
    M: AsRef<[u8]> + AsMut<[u8]>,
{
    type Record = T;

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn length(&self) -> usize {
        self.length
    }

    fn remaining(&self) -> usize {
        self.capacity - self.length
    }

    fn trim(&mut self, len: usize) {
        let offset = len * T::size();
        let end_offset = self.length * T::size();

        // Safety: Invariant; len <= self.len()
        unsafe {
            // Reference to bytes held in storage.
            let mem = self.mem.as_mut();

            // Pointers to addresses we will copy to/from.
            let dst_ptr = mem.as_mut_ptr();
            let src_ptr = mem.as_ptr().add(offset);

            // Has to be memove rather than memcpy because we are copying overlapping
            // range of bytes. This generally requires memory to be copied in certain
            // direction, unlike memcpy that can arbitrarily copy bytes.
            std::ptr::copy(src_ptr, dst_ptr, end_offset - offset);
        }

        self.length -= len;
    }

    fn extend(&mut self, records: &[T]) {
        let offset = self.length * T::size();
        let src = T::to_bytes_slice(records);

        // Safety: Invariant; records.len() <= self.remaining()
        unsafe {
            // Reference to bytes held in storage.
            let mem = self.mem.as_mut();

            // Pointers to addresses we will copy to/from.
            let src_ptr = src.as_ptr();
            let dst_ptr = mem.as_mut_ptr().add(offset);

            // Source and destination are guaranteed to be separate memory allocations,
            // meaning they don't share the same memory regions. So it's safe to use
            // memcpy here to copy bytes from source to destination.
            std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, src.len());
        }

        self.length += records.len();
    }

    fn clear(&mut self) {
        self.length = 0;
    }

    fn records(&self) -> &[T] {
        // Safety: Other invariants that make sure length tracked is correct.
        unsafe {
            // Reference to bytes held in storage.
            let mem = self.mem.as_ref();

            // Find the range of bytes to read.
            let mid = self.length * T::size();
            let (bytes, _) = mem.split_at_unchecked(mid);

            // Transmute bytes to records.
            T::from_bytes_slice(bytes)
        }
    }
}

/// Off heap memory that backs a [`MemStorage`] engine.
#[derive(Debug)]
pub struct OffHeap(MmapMut);

impl OffHeap {
    /// Allocate some number of bytes on heap.
    ///
    /// * Frees memory using RAII pattern, so no method to deallocate memory.
    /// * If successful memory is guaranteed to be page aligned.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of bytes to allocate.
    fn alloc(capacity: usize) -> Self {
        let mmap = MmapOptions::new()
            // .huge(None) TODO: Enable support for huge pages.
            .len(capacity)
            // Fault all pages so that they are eagerly initialized.
            .populate()
            // Map with anonymous memory map for off-heap memory.
            .map_anon()
            // Especially with huge pages.
            .expect("Cannot allocate anonymous mmap");

        Self(mmap)
    }
}

impl AsRef<[u8]> for OffHeap {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for OffHeap {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

/// On heap memory that backs a [`MemStorage`] engine.
#[derive(Debug)]
pub struct OnHeap {
    ptr: *mut u8,
    layout: Layout,
    len: usize,
}

impl OnHeap {
    /// Allocate some number of bytes on heap.
    ///
    /// Frees memory using RAII pattern, so no method to deallocate memory.
    ///
    /// Note, if successful we might over allocate, i.e contain more bytes than
    /// requested. But this will never be visible outside of this container.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of bytes to allocate.
    fn alloc(capacity: usize) -> Self {
        // Layout the describes the allocation requirements.
        let align = align_of::<u8>();
        let layout = Layout::from_size_align(capacity, align)
            .expect("Cannot create a layout for global allocator");

        // Safety
        // 1. We are properly aligning memory (which should be 1).
        // 2. Size of allocation must be > 0 (cannot create layout otherwise).
        let ptr = unsafe {
            // Allocate memory.
            let ptr = alloc::alloc(layout);

            // If allocation was unsuccessful.
            if ptr.is_null() {
                handle_alloc_error(layout);
            }

            // Return pointer to the newly allocated memory.
            // This is now guaranteed to be non-null.
            ptr
        };

        Self {
            ptr,
            layout,
            len: capacity,
        }
    }
}

impl Drop for OnHeap {
    fn drop(&mut self) {
        // Cannot initialize with invalid pointer and layout.
        unsafe {
            alloc::dealloc(self.ptr, self.layout);
        }
    }
}

impl AsRef<[u8]> for OnHeap {
    fn as_ref(&self) -> &[u8] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl AsMut<[u8]> for OnHeap {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety
        // * Pointer is guaranteed to be initialized.
        // * length is guaranteed to be > 0.
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}
