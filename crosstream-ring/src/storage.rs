//! Definition of storage engine that backs ring buffer.

use crate::Record;
use memmap2::{MmapMut, MmapOptions};
use std::marker::PhantomData;

/// Storage engine that holds a contiguous sequence of records.
///
/// # Internal
///
/// For the most part this is a trait that is internal to this crate. It's only
/// exposed externally because this is part of type signature for types that are
/// exposed externally. If you are trying to implement this trait outside of the
/// crate, you are probably doing something wrong.
pub trait Storage {
    /// Associated type for records stored.
    type Record: Record;

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

/// Storage engine for [`Segment`] that uses [`Vec`] for memory.
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

impl<T: Record + Copy> Storage for VecStorage<T> {
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
        self.0.extend_from_slice(records);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn records(&self) -> &[T] {
        &self.0
    }
}

/// Storage engine for [`Segment`] that uses [`MmapMut`] for memory.
#[derive(Debug)]
pub struct MmapStorage<T> {
    mmap: MmapMut,
    length: usize,
    capacity: usize,
    phantom: PhantomData<T>,
}

impl<T: Record> MmapStorage<T> {
    /// Create a new instance of [`MmapMut`] backed [`Storage`] engine.
    ///
    /// # Panics
    ///
    /// Panics if requested capacity could not be allocated.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum capacity of this storage engine.
    pub(crate) fn new(capacity: usize) -> Self {
        let mmap = MmapOptions::new()
            // .huge(None) TODO: Enable support for huge pages.
            .len(capacity * T::size())
            .populate()
            .map_anon()
            .expect("Cannot mmap capacity");

        Self {
            mmap,
            length: 0,
            capacity,
            phantom: PhantomData,
        }
    }
}

impl<T: Record> Storage for MmapStorage<T> {
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
        self.mmap.copy_within((len * T::size()).., 0);
        self.length -= len;
    }

    fn extend(&mut self, records: &[T]) {
        let offset = self.length * T::size();
        let src = T::to_bytes_slice(records);
        let dst = &mut self.mmap[offset..(offset + src.len())];
        dst.copy_from_slice(src);
        self.length += records.len();
    }

    fn clear(&mut self) {
        self.length = 0;
    }

    fn records(&self) -> &[T] {
        let end = self.length * T::size();
        T::from_bytes_slice(&self.mmap[..end])
    }
}
