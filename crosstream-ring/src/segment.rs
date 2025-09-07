//! Definition of a container of contiguous elements.

use crate::Record;
use memmap2::{MmapMut, MmapOptions};
use std::cmp::min;
use std::marker::PhantomData;

/// Type alias for a [`Segment`] backed by a [`VecStorage`].
pub type VecSegment<T> = Segment<VecStorage<T>>;

/// Type alias for a [`Segment`] backed by a [`MmapStorage`].
pub type MmapSegment<T> = Segment<MmapStorage<T>>;

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
/// * Provides two storage engines: [`VecStorage`] and [`MmapStorage`].
#[derive(Debug)]
pub struct Segment<S: Storage> {
    length: usize,
    capacity: usize,
    storage: S,
    trimmer: Trimmer,
}

impl<S: Storage> Segment<S> {
    /// Number of records currently stored in this Segment.
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Maximum number of records that can be stored in this Segment.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of records that can be appended to this Segment without overflow.
    #[inline]
    pub fn remaining(&self) -> usize {
        self.capacity - self.length
    }

    /// true if this Segment has no records, false otherwise.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// true if this Segment is at capacity, false otherwise.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.capacity == self.length
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
        if len >= self.length {
            self.clear();
            return;
        }

        // We need to left shift some bytes.
        self.storage.trim(len);
        self.length -= len;
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
    pub fn push(&mut self, record: S::Record) -> Option<S::Record> {
        // If we don't have enough capacity, attempt to trim records.
        if self.remaining() == 0 {
            self.run_trimmer();
        }

        // If we still don't have enough space, there is nothing else to do.
        if self.remaining() == 0 {
            return Some(record);
        }

        // Copy record bytes to internal buffers.
        self.storage.extend(self.length, &[record]);
        self.length += 1;

        // The record was consumed, nothing to return.
        None
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
    pub fn extend_from_slice<'a>(&mut self, records: &'a [S::Record]) -> &'a [S::Record] {
        // If we don't have enough capacity, attempt to trim records.
        if self.remaining() < records.len() {
            self.run_trimmer();
        }

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
        self.storage.extend(self.length, to_append);
        self.length += to_append.len();

        // Return all the rejected records.
        to_reject
    }

    /// Remove all elements from this Segment.
    ///
    /// * This is a constant time O(1) operation.
    #[inline]
    pub fn clear(&mut self) {
        self.storage.clear();
        self.length = 0;
    }

    /// Returns reference to all the records in a segment.
    #[inline]
    pub fn records(&self) -> &[S::Record] {
        self.storage.records(self.length)
    }

    #[inline]
    fn run_trimmer(&mut self) {
        let trim_len = match self.trimmer {
            Trimmer::Nothing => 0,
            Trimmer::Trim(len) => len,
        };

        if trim_len > 0 {
            self.trim(trim_len);
        }
    }
}

/// Storage engine that backs a [`Segment`].
pub trait Storage {
    /// Associated type for records stored.
    type Record: Record;

    /// Trim first len records from storage.
    fn trim(&mut self, len: usize);

    /// Append some records into storage.
    fn extend(&mut self, len: usize, records: &[Self::Record]);

    /// Clear all records from storage.
    fn clear(&mut self);

    /// Return reference to all records in storage.
    fn records(&self, len: usize) -> &[Self::Record];
}

/// Storage engine for [`Segment`] that uses [`Vec`] for memory.
#[derive(Debug)]
pub struct VecStorage<T>(Vec<T>);

impl<T: Record + Copy> VecSegment<T> {
    /// Create a new instance of Segment using [`Vec`] for memory.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this segment can accommodate.
    /// * `trimmer` - Trimmer to use when appending records into segment.
    pub fn with_capacity(capacity: usize, trimmer: Trimmer) -> VecSegment<T> {
        Self {
            length: 0,
            capacity,
            trimmer,
            storage: VecStorage(Vec::with_capacity(capacity)),
        }
    }
}

impl<T: Record + Copy> Storage for VecStorage<T> {
    type Record = T;

    #[inline]
    fn trim(&mut self, len: usize) {
        self.0.drain(..min(len, self.0.len()));
    }

    #[inline]
    fn extend(&mut self, _len: usize, records: &[T]) {
        self.0.extend_from_slice(records);
    }

    #[inline]
    fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    fn records(&self, _len: usize) -> &[T] {
        &self.0
    }
}

/// Storage engine for [`Segment`] that uses [`MmapMut`] for memory.
#[derive(Debug)]
pub struct MmapStorage<T> {
    mmap: MmapMut,
    phantom: PhantomData<T>,
}

impl<T: Record + Copy> MmapSegment<T> {
    /// Create a new instance of Segment using [`MmapMut`] for memory.
    ///
    /// * TODO: Add support for huge pages.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this segment can accommodate.
    /// * `trimmer` - Trimmer to use when appending records into segment.
    pub fn with_capacity(capacity: usize, trimmer: Trimmer) -> MmapSegment<T> {
        let mmap = MmapOptions::new()
            // .huge(None) TODO: Enable support for huge pages.
            .len(capacity * T::size())
            .populate()
            .map_anon()
            .expect("Cannot mmap capacity");

        Self {
            length: 0,
            trimmer,
            capacity,
            storage: MmapStorage {
                mmap,
                phantom: PhantomData,
            },
        }
    }
}

impl<T: Record> Storage for MmapStorage<T> {
    type Record = T;

    #[inline]
    fn trim(&mut self, len: usize) {
        self.mmap.copy_within((len * T::size()).., 0);
    }

    #[inline]
    fn extend(&mut self, len: usize, records: &[T]) {
        let offset = len * T::size();
        let src = T::to_bytes_slice(records);
        let dst = &mut self.mmap[offset..(offset + src.len())];
        dst.copy_from_slice(src);
    }

    #[inline]
    fn clear(&mut self) {}

    #[inline]
    fn records(&self, len: usize) -> &[T] {
        let end = len * T::size();
        T::from_bytes_slice(&self.mmap[..end])
    }
}

/// Strategy used to trim records during appends into a [`Segment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub enum Trimmer {
    /// When an append operation occurs and there isn't sufficient capacity
    /// to accommodate records, this does nothing. Meaning one or more records
    /// might be rejected from the segment.
    Nothing,

    /// When an append operation occurs and there isn't sufficient capacity
    /// to accommodate records, this trims first N records from the segment.
    /// However records will  be rejected if N < number of records being appended.
    Trim(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::{check, generator::*};
    use std::{fmt::Debug, time::Duration};

    // Anout 100 MB worth of memory.
    const SEG_MEMORY: usize = 100 * 1024 * 1024;

    #[derive(Debug, TypeGenerator)]
    enum Operation<T: TypeGenerator + Debug> {
        Clear,
        Trim(u8),
        Push(T),
        Extend(Vec<T>),
    }

    macro_rules! state_machine_test {
        ($name:ident,$num:ty) => {
            #[test]
            fn $name() {
                check!()
                    .with_iterations(50)
                    .with_test_time(Duration::from_millis(500))
                    .with_type::<(Vec<Operation<$num>>, Trimmer)>()
                    .for_each(|(operations, trimmer)| {
                        let capacity = SEG_MEMORY / <$num>::size();
                        let mut mmap = MmapSegment::with_capacity(capacity, *trimmer);
                        let mut vec = VecSegment::with_capacity(capacity, *trimmer);

                        for operation in operations {
                            match operation {
                                Operation::Clear => {
                                    mmap.clear();
                                    vec.clear();
                                }

                                Operation::Trim(len) => {
                                    mmap.trim(*len as usize);
                                    vec.trim(*len as usize);
                                }

                                Operation::Push(record) => {
                                    mmap.push(*record);
                                    vec.push(*record);
                                }

                                Operation::Extend(records) => {
                                    mmap.extend_from_slice(records);
                                    vec.extend_from_slice(records);
                                }
                            }

                            assert_eq!(mmap.records(), vec.records());
                        }
                    });
            }
        };
    }

    state_machine_test!(state_machine_u8, u8);
    state_machine_test!(state_machine_u16, u16);
    state_machine_test!(state_machine_u32, u32);
    state_machine_test!(state_machine_u64, u64);
    state_machine_test!(state_machine_u128, u128);
    state_machine_test!(state_machine_usize, usize);

    state_machine_test!(state_machine_i8, i8);
    state_machine_test!(state_machine_i16, i16);
    state_machine_test!(state_machine_i32, i32);
    state_machine_test!(state_machine_i64, i64);
    state_machine_test!(state_machine_i128, i128);
    state_machine_test!(state_machine_isize, isize);

    state_machine_test!(state_machine_u8_2, [u8; 2]);
    state_machine_test!(state_machine_i8_2, [i8; 2]);

    state_machine_test!(state_machine_u8_4, [u8; 4]);
    state_machine_test!(state_machine_i8_4, [i8; 4]);

    state_machine_test!(state_machine_u8_8, [u8; 8]);
    state_machine_test!(state_machine_i8_8, [i8; 8]);

    state_machine_test!(state_machine_u8_16, [u8; 16]);
    state_machine_test!(state_machine_i8_16, [i8; 16]);

    state_machine_test!(state_machine_u8_32, [u8; 32]);
    state_machine_test!(state_machine_i8_32, [i8; 32]);

    state_machine_test!(state_machine_u8_64, [u8; 64]);
    state_machine_test!(state_machine_i8_64, [i8; 64]);

    state_machine_test!(state_machine_u8_3, [u8; 3]);
    state_machine_test!(state_machine_i8_3, [i8; 3]);

    state_machine_test!(state_machine_u8_9, [u8; 9]);
    state_machine_test!(state_machine_i8_9, [i8; 9]);

    state_machine_test!(state_machine_u8_27, [u8; 27]);
    state_machine_test!(state_machine_i8_27, [i8; 27]);

    state_machine_test!(state_machine_u8_81, [u8; 81]);
    state_machine_test!(state_machine_i8_81, [i8; 81]);

    state_machine_test!(state_machine_u8_5, [u8; 5]);
    state_machine_test!(state_machine_i8_5, [i8; 5]);

    state_machine_test!(state_machine_u8_25, [u8; 25]);
    state_machine_test!(state_machine_i8_25, [i8; 25]);

    state_machine_test!(state_machine_u8_125, [u8; 125]);
    state_machine_test!(state_machine_i8_125, [i8; 125]);

    state_machine_test!(state_machine_u16_1, [u16; 1]);
    state_machine_test!(state_machine_u16_2, [u16; 2]);
    state_machine_test!(state_machine_u16_3, [u16; 3]);
    state_machine_test!(state_machine_u16_4, [u16; 4]);
    state_machine_test!(state_machine_u16_5, [u16; 5]);
    state_machine_test!(state_machine_u16_6, [u16; 6]);
    state_machine_test!(state_machine_u16_7, [u16; 7]);
    state_machine_test!(state_machine_u16_8, [u16; 8]);
    state_machine_test!(state_machine_u16_19, [u16; 9]);
    state_machine_test!(state_machine_u16_10, [u16; 10]);
    state_machine_test!(state_machine_u16_11, [u16; 11]);
    state_machine_test!(state_machine_u16_12, [u16; 12]);
    state_machine_test!(state_machine_u16_13, [u16; 13]);
    state_machine_test!(state_machine_u16_14, [u16; 14]);
    state_machine_test!(state_machine_u16_15, [u16; 15]);
    state_machine_test!(state_machine_u16_16, [u16; 16]);

    state_machine_test!(state_machine_u32_1, [u32; 1]);
    state_machine_test!(state_machine_u32_2, [u32; 2]);
    state_machine_test!(state_machine_u32_3, [u32; 3]);
    state_machine_test!(state_machine_u32_4, [u32; 4]);
    state_machine_test!(state_machine_u32_5, [u32; 5]);
    state_machine_test!(state_machine_u32_6, [u32; 6]);
    state_machine_test!(state_machine_u32_7, [u32; 7]);
    state_machine_test!(state_machine_u32_8, [u32; 8]);

    state_machine_test!(state_machine_u64_1, [u64; 1]);
    state_machine_test!(state_machine_u64_2, [u64; 2]);
    state_machine_test!(state_machine_u64_3, [u64; 3]);
    state_machine_test!(state_machine_u64_4, [u64; 4]);

    state_machine_test!(state_machine_usize_1, [usize; 1]);
    state_machine_test!(state_machine_usize_2, [usize; 2]);
    state_machine_test!(state_machine_usize_3, [usize; 3]);
    state_machine_test!(state_machine_usize_4, [usize; 4]);

    state_machine_test!(state_machine_u128_1, [u128; 1]);
    state_machine_test!(state_machine_u128_2, [u128; 2]);

    state_machine_test!(state_machine_i16_1, [i16; 1]);
    state_machine_test!(state_machine_i16_2, [i16; 2]);
    state_machine_test!(state_machine_i16_3, [i16; 3]);
    state_machine_test!(state_machine_i16_4, [i16; 4]);
    state_machine_test!(state_machine_i16_5, [i16; 5]);
    state_machine_test!(state_machine_i16_6, [i16; 6]);
    state_machine_test!(state_machine_i16_7, [i16; 7]);
    state_machine_test!(state_machine_i16_8, [i16; 8]);
    state_machine_test!(state_machine_i16_19, [i16; 9]);
    state_machine_test!(state_machine_i16_10, [i16; 10]);
    state_machine_test!(state_machine_i16_11, [i16; 11]);
    state_machine_test!(state_machine_i16_12, [i16; 12]);
    state_machine_test!(state_machine_i16_13, [i16; 13]);
    state_machine_test!(state_machine_i16_14, [i16; 14]);
    state_machine_test!(state_machine_i16_15, [i16; 15]);
    state_machine_test!(state_machine_i16_16, [i16; 16]);

    state_machine_test!(state_machine_i32_1, [i32; 1]);
    state_machine_test!(state_machine_i32_2, [i32; 2]);
    state_machine_test!(state_machine_i32_3, [i32; 3]);
    state_machine_test!(state_machine_i32_4, [i32; 4]);
    state_machine_test!(state_machine_i32_5, [i32; 5]);
    state_machine_test!(state_machine_i32_6, [i32; 6]);
    state_machine_test!(state_machine_i32_7, [i32; 7]);
    state_machine_test!(state_machine_i32_8, [i32; 8]);

    state_machine_test!(state_machine_i64_1, [i64; 1]);
    state_machine_test!(state_machine_i64_2, [i64; 2]);
    state_machine_test!(state_machine_i64_3, [i64; 3]);
    state_machine_test!(state_machine_i64_4, [i64; 4]);

    state_machine_test!(state_machine_isize_1, [isize; 1]);
    state_machine_test!(state_machine_isize_2, [isize; 2]);
    state_machine_test!(state_machine_isize_3, [isize; 3]);
    state_machine_test!(state_machine_isize_4, [isize; 4]);

    state_machine_test!(state_machine_i128_1, [i128; 1]);
    state_machine_test!(state_machine_i128_2, [i128; 2]);
}
