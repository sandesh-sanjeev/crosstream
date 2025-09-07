//! Definition of storage engine that backs a Ring.

use crate::Record;
use memmap2::{MmapMut, MmapOptions};
use std::cmp::min;
use std::io;
use std::{borrow::Borrow, marker::PhantomData, ops::Deref};

/// Segment is a container of contiguous elements.
///
/// The intended purpose for this is as a building block for a high performance off-heap
/// ring buffer. But might be useful in other cases that requires large amounts of data
/// in memory (maybe on disk on day).
///
/// * Memory is allocated using anonymous mmap rather than global allocator.
/// * Does not support growth, i.e, cannot be resized to increase capacity.
/// * There can be no gaps between elements, push/remove from back, but only remove from front.
/// * As of now (and probably forever) only supports elements that implement [`Record`].
/// * Performance of `Segment` operations is virtually identical to that of [`Vec`].
#[derive(Debug)]
pub struct Segment<T> {
    len: usize,
    cap: usize,
    memory: MmapMut,
    phantom: PhantomData<T>,
}

impl<T: Record> Segment<T> {
    /// Create a new instance of Segment.
    ///
    /// * TODO: Add support for huge pages.
    ///
    /// Note that this variant panics when memory cannot be allocated via mmap.
    /// For a non-panicking alternative, use [`Segment::try_with_capacity`].
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this ring can accommodate.
    pub fn with_capacity(capacity: usize) -> Self {
        match Self::try_with_capacity(capacity) {
            Ok(ring) => ring,
            Err(e) => panic!("Error allocating memory for ring: {e}"),
        }
    }

    /// Create a new instance of Segment.
    ///
    /// * Returns an I/O error if memory allocation fails.
    /// * TODO: Add support for huge pages.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this ring can accommodate.
    pub fn try_with_capacity(capacity: usize) -> io::Result<Self> {
        let mmap = MmapOptions::new()
            // .huge(None) TODO: Enable support for huge pages.
            .len(capacity * T::size())
            .populate()
            .map_anon()?;

        Ok(Self {
            len: 0,
            cap: capacity,
            memory: mmap,
            phantom: PhantomData,
        })
    }

    /// Number of records currently stored in this Segment.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Maximum number of records that can be stored in this Segment.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Number of records that can be appended to this Segment without overflow.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.cap - self.len
    }

    /// true if this Segment has no records, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// true if this Segment is at capacity, false otherwise.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.cap == self.len
    }

    /// Trim first N records from this Segment.
    ///
    /// * If N == 0, this operation is no-op.
    /// * If N > self.len(), this operation is alias for [`Segment::clear`].
    /// * This is an O(M) operation where M = self.len() - N.
    ///
    /// Like a [`Vec`] removing elements from the front of the Segment is not efficient.
    /// For small-ish Segments, and/or if trims are infrequent this is probably okay.
    /// See what works for you by benchmarking with your use case.
    ///
    /// # Arguments
    ///
    /// * `len` - Number of records to remove.
    #[inline]
    pub fn trim(&mut self, len: usize) {
        // Early return if there is nothing to trim.
        if len == 0 {
            return;
        }

        // Optimization if all the records can be trimmed.
        if len >= self.len {
            self.clear();
            return;
        }

        // We need to left shift some bytes.
        self.memory.copy_within((len * T::size()).., 0);
        self.len -= len;
    }

    /// Append a record into this Segment.
    ///
    /// * Returns records that were rejected due to overflow.
    /// * This is a constant time O(1) operation.
    ///
    /// # Arguments
    ///
    /// * `record` - Record to append.
    #[inline]
    pub fn push(&mut self, record: T) -> bool {
        // Early return when there is no capacity for the record.
        if self.remaining() == 0 {
            return false;
        }

        // Copy record bytes to internal buffers.
        let offset = self.len * T::size();
        let src = T::to_bytes(record.borrow());
        let dst = &mut self.memory[offset..(offset + src.len())];
        dst.copy_from_slice(src);
        self.len += 1;

        // Indicate that record was accepted.
        true
    }

    /// Append a slice of records into this Segment.
    ///
    /// * Returns records that were rejected due to overflow.
    /// * This is a constant time O(N) operation, where N is records.len().
    ///
    /// # Arguments
    ///
    /// * `records` - Records to append.
    #[inline]
    pub fn extend_from_slice<'a>(&mut self, records: &'a [T]) -> &'a [T] {
        // Safety: index is guaranteed to be <= records.len() due to the conditional check.
        let (to_append, to_reject) = unsafe {
            let index = min(records.len(), self.remaining());
            records.split_at_unchecked(index)
        };

        // Early return when there is no capacity for any records.
        if to_append.is_empty() {
            return to_reject;
        }

        // Copy record bytes to internal buffers.
        let offset = self.len * T::size();
        let src = T::to_bytes_slice(to_append);
        let dst = &mut self.memory[offset..(offset + src.len())];
        dst.copy_from_slice(src);
        self.len += to_append.len();

        // Return all the rejected records.
        to_reject
    }

    /// Remove all elements from this Segment.
    ///
    /// * This is a constant time O(1) operation.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T: Record> Deref for Segment<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        let end = self.len * T::size();
        T::from_bytes_slice(&self.memory[..end])
    }
}

/// Storage engine that backs a [`Segment`].
trait Storage<T: Record> {
    fn trim(&mut self, len: usize);

    fn extend(&mut self, len: usize, records: &[T]);

    fn clear(&mut self);

    fn records(&self, len: usize) -> &[T];
}

/// Storage engine for [`Segment`] that uses heap allocated  mmap for storage.
#[derive(Debug)]
pub struct VecStorage<T>(Vec<T>);

impl<T: Record + Copy> Storage<T> for VecStorage<T> {
    fn trim(&mut self, len: usize) {
        self.0.drain(..min(len, self.0.len()));
    }

    fn extend(&mut self, _len: usize, records: &[T]) {
        self.0.extend_from_slice(records);
    }

    fn clear(&mut self) {
        self.0.clear();
    }

    fn records(&self, _len: usize) -> &[T] {
        &self.0
    }
}

/// Storage engine for [`Segment`] that uses anonymous mmap for storage.
#[derive(Debug)]
pub struct MmapStorage<T> {
    mmap: MmapMut,
    phantom: PhantomData<T>,
}

impl<T: Record> Storage<T> for MmapStorage<T> {
    fn trim(&mut self, len: usize) {
        self.mmap.copy_within((len * T::size()).., 0);
    }

    fn extend(&mut self, len: usize, records: &[T]) {
        let offset = len * T::size();
        let src = T::to_bytes_slice(records);
        let dst = &mut self.mmap[offset..(offset + src.len())];
        dst.copy_from_slice(src);
    }

    fn clear(&mut self) {}

    fn records(&self, len: usize) -> &[T] {
        let end = len * T::size();
        T::from_bytes_slice(&self.mmap[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::{check, generator::*};

    /// Maximum capacity of the test ring buffer.
    const RING_CAPACITY: usize = 1024 * 1024;

    /// Methods of a ring buffer being tested.
    trait Ring<T> {
        /// Clear ring buffer.
        fn test_clear(&mut self);

        /// Trim some number of records from front of the ring buffer.
        fn test_trim(&mut self, len: &u8);

        /// Append a record into the ring buffer.
        fn test_push(&mut self, record: &T);

        /// Append a slice of records into the ring buffer.
        fn test_extend_slice(&mut self, records: &[T]);

        /// Get a reference to records held in the ring buffer.
        fn test_records(&self) -> &[T];
    }

    // Reference implementation of ring buffer using a Vec.
    impl<T: Copy> Ring<T> for Vec<T> {
        fn test_clear(&mut self) {
            self.clear();
        }

        fn test_trim(&mut self, len: &u8) {
            self.drain(..min(*len as usize, self.len()));
        }

        fn test_push(&mut self, record: &T) {
            if self.len() >= RING_CAPACITY {
                self.remove(0);
            }

            self.push(*record);
        }

        fn test_extend_slice(&mut self, records: &[T]) {
            if records.len() > RING_CAPACITY {
                return;
            }

            let remaining = RING_CAPACITY - records.len();
            if remaining < records.len() {
                self.drain(..(records.len() - remaining));
            }

            self.extend_from_slice(records);
        }

        fn test_records(&self) -> &[T] {
            &self
        }
    }

    // Implementation of ring buffer using `Ring`.
    impl<T: Record + Copy> Ring<T> for Segment<T> {
        fn test_clear(&mut self) {
            self.clear();
        }

        fn test_trim(&mut self, len: &u8) {
            self.trim(*len as usize);
        }

        fn test_push(&mut self, record: &T) {
            if self.is_full() {
                self.trim(1);
            }

            self.push(*record);
        }

        fn test_extend_slice(&mut self, records: &[T]) {
            if records.len() > RING_CAPACITY {
                return;
            }

            let remaining = self.remaining();
            if remaining < records.len() {
                self.trim(records.len() - remaining);
            }

            self.extend_from_slice(records);
        }

        fn test_records(&self) -> &[T] {
            &self
        }
    }

    macro_rules! state_machine_test {
        ($name:ident, $operation:ident, $num:ty) => {
            #[derive(Debug, TypeGenerator)]
            enum $operation {
                Clear,
                Trim(u8),
                Push($num),
                Extend(Vec<$num>),
            }

            #[test]
            fn $name() {
                check!()
                    .with_type::<Vec<$operation>>()
                    .for_each(|operations| {
                        let mut ring = Segment::with_capacity(RING_CAPACITY);
                        let mut vec = Vec::with_capacity(RING_CAPACITY);

                        for operation in operations {
                            match operation {
                                $operation::Clear => {
                                    ring.test_clear();
                                    vec.test_clear();
                                }

                                $operation::Trim(len) => {
                                    ring.test_trim(len);
                                    vec.test_trim(len);
                                }

                                $operation::Push(record) => {
                                    ring.test_push(record);
                                    vec.test_push(record);
                                }

                                $operation::Extend(records) => {
                                    ring.test_extend_slice(records);
                                    vec.test_extend_slice(records);
                                }
                            }

                            assert_eq!(ring.test_records(), vec.test_records());
                        }
                    })
            }
        };
    }

    state_machine_test!(state_machine_u8, OperationU8, u8);
    state_machine_test!(state_machine_u16, OperationU16, u16);
    state_machine_test!(state_machine_u32, OperationU32, u32);
    state_machine_test!(state_machine_u64, OperationU64, u64);
    state_machine_test!(state_machine_u128, OperationU128, u128);
    state_machine_test!(state_machine_usize, OperationUsize, usize);

    state_machine_test!(state_machine_i8, OperationI8, i8);
    state_machine_test!(state_machine_i16, OperationI16, i16);
    state_machine_test!(state_machine_i32, OperationI32, i32);
    state_machine_test!(state_machine_i64, OperationI64, i64);
    state_machine_test!(state_machine_i128, OperationI128, i128);
    state_machine_test!(state_machine_isize, OperationIsize, isize);
}
