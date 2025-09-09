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
    pub fn len(&self) -> usize {
        self.length
    }

    /// Maximum number of records that can be stored in this Segment.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of records that can be appended to this Segment without overflow.
    pub fn remaining(&self) -> usize {
        self.capacity - self.length
    }

    /// true if this Segment has no records, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// true if this Segment is at capacity, false otherwise.
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
    pub fn clear(&mut self) {
        self.storage.clear();
        self.length = 0;
    }

    /// Returns reference to all the records in a segment.
    pub fn records(&self) -> &[S::Record] {
        self.storage.records(self.length)
    }

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

/// Strategy used to trim records during appends into a [`Segment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
#[cfg_attr(kani, derive(kani::Arbitrary))]
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

#[cfg(kani)]
mod verification {
    use super::*;

    const CAPACITY: usize = 10;

    #[kani::proof]
    #[kani::unwind(512)]
    fn push_proof() {
        let mut segment = VecSegment::with_capacity(CAPACITY, Trimmer::Nothing);

        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());
        let records: [u64; CAPACITY] = kani::any();

        // Write individual records.
        assert_eq!(0, segment.len());
        for record in &records {
            assert!(segment.remaining() > 0);
            assert!(segment.push(*record).is_none());
            assert_eq!(&records[..segment.len()], segment.records());
        }

        // Make sure segment has all the records now.
        assert!(segment.is_full());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        let one_more = kani::any();
        assert_eq!(segment.push(one_more), Some(one_more));

        // Trim and get back some space.
        segment.trim(1);
        assert_eq!(segment.push(one_more), None);
        assert_eq!(segment.records()[..CAPACITY - 1], records[1..]);
        assert_eq!(segment.records()[CAPACITY - 1], one_more);
    }

    #[kani::proof]
    #[kani::unwind(512)]
    fn push_with_trim_proof() {
        let trim = 4;
        let mut segment = VecSegment::with_capacity(CAPACITY, Trimmer::Trim(trim));

        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());

        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);
        assert!(segment.extend_from_slice(&records).is_empty());
        assert_eq!(&records, segment.records());

        // Write one more, it should automatically create space for 100 more.
        let one_more = kani::any();
        assert_eq!(segment.push(one_more), None);

        // Make sure expected state.
        assert_eq!(&segment.records()[..CAPACITY - trim], &records[trim..]);
        assert_eq!(segment.records()[CAPACITY - trim], one_more);
        assert_eq!(trim - 1, segment.remaining());
        assert_eq!(segment.len(), CAPACITY - trim + 1);
    }

    #[kani::proof]
    #[kani::unwind(512)]
    fn extend_from_slice_proof() {
        let mut segment = VecSegment::with_capacity(CAPACITY, Trimmer::Nothing);

        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());
        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);

        // Split into multiple chunks and write to segment.
        for chunk in records.chunks(61) {
            assert!(segment.remaining() >= chunk.len());
            assert!(segment.extend_from_slice(chunk).is_empty());
            assert_eq!(&records[..segment.len()], segment.records());
        }

        // Make sure segment has all the records now.
        assert!(segment.is_full());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        let more_records = [3, 5, 7, 9];
        assert_eq!(segment.extend_from_slice(&more_records), &more_records);

        // Trim and get back some space.
        segment.trim(3);
        assert_eq!(segment.extend_from_slice(&more_records), &more_records[3..]);
        assert_eq!(&segment.records()[..CAPACITY - 3], &records[3..]);
        assert_eq!(&segment.records()[CAPACITY - 3..], &more_records[..3]);
    }

    #[kani::proof]
    #[kani::unwind(512)]
    fn extend_from_slice_with_trim_proof() {
        let trim = 4;
        let mut segment = VecSegment::with_capacity(CAPACITY, Trimmer::Trim(trim));

        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());

        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);
        assert!(segment.extend_from_slice(&records).is_empty());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        let more_records: [_; CAPACITY] = std::array::from_fn(|i| i * 3);
        let rejected = segment.extend_from_slice(&more_records);

        // Everything more than how many records we trimmed should be rejected.
        let trimmed = std::cmp::min(CAPACITY, trim);
        assert_eq!(rejected, &more_records[trimmed..]);

        // Make sure expected state.
        assert_eq!(0, segment.remaining());
        assert_eq!(segment.len(), CAPACITY);
        assert_eq!(
            &segment.records()[..CAPACITY - trimmed],
            &records[trimmed..]
        );
        assert_eq!(
            &segment.records()[CAPACITY - trimmed..],
            &more_records[..trimmed]
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const CAPACITY: usize = 1024;

    fn mmap_segment(trimmer: Trimmer) -> MmapSegment<usize> {
        MmapSegment::with_capacity(CAPACITY, trimmer)
    }

    fn vec_segment(trimmer: Trimmer) -> VecSegment<usize> {
        VecSegment::with_capacity(CAPACITY, trimmer)
    }

    #[rstest]
    #[case(vec_segment(Trimmer::Nothing))]
    #[case(mmap_segment(Trimmer::Nothing))]
    fn test_push<S: Storage<Record = usize>>(#[case] mut segment: Segment<S>) {
        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());
        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);

        // Write individual records.
        assert_eq!(0, segment.len());
        for record in &records {
            assert!(segment.remaining() > 0);
            assert!(segment.push(*record).is_none());
            assert_eq!(&records[..segment.len()], segment.records());
        }

        // Make sure segment has all the records now.
        assert!(segment.is_full());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        assert_eq!(segment.push(100), Some(100));

        // Trim and get back some space.
        segment.trim(1);
        assert_eq!(segment.push(100), None);
        assert_eq!(segment.records()[..CAPACITY - 1], records[1..]);
        assert_eq!(segment.records()[CAPACITY - 1], 100);
    }

    #[rstest]
    #[case(vec_segment(Trimmer::Trim(100)), 100)]
    #[case(mmap_segment(Trimmer::Trim(100)), 100)]
    fn test_push_trimmer<S: Storage<Record = usize>>(
        #[case] mut segment: Segment<S>,
        #[case] trim: usize,
    ) {
        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());

        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);
        assert!(segment.extend_from_slice(&records).is_empty());
        assert_eq!(&records, segment.records());

        // Write one more, it should automatically create space for 100 more.
        assert_eq!(segment.push(100), None);

        // Make sure expected state.
        assert_eq!(&segment.records()[..CAPACITY - trim], &records[trim..]);
        assert_eq!(segment.records()[CAPACITY - trim], 100);
        assert_eq!(trim - 1, segment.remaining());
        assert_eq!(segment.len(), CAPACITY - trim + 1);
    }

    #[rstest]
    #[case(vec_segment(Trimmer::Nothing))]
    #[case(mmap_segment(Trimmer::Nothing))]
    fn test_extend_from_slice<S: Storage<Record = usize>>(#[case] mut segment: Segment<S>) {
        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());
        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);

        // Split into multiple chunks and write to segment.
        for chunk in records.chunks(61) {
            assert!(segment.remaining() >= chunk.len());
            assert!(segment.extend_from_slice(chunk).is_empty());
            assert_eq!(&records[..segment.len()], segment.records());
        }

        // Make sure segment has all the records now.
        assert!(segment.is_full());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        let more_records = [3, 5, 7, 9];
        assert_eq!(segment.extend_from_slice(&more_records), &more_records);

        // Trim and get back some space.
        segment.trim(3);
        assert_eq!(segment.extend_from_slice(&more_records), &more_records[3..]);
        assert_eq!(&segment.records()[..CAPACITY - 3], &records[3..]);
        assert_eq!(&segment.records()[CAPACITY - 3..], &more_records[..3]);
    }

    #[rstest]
    #[case(vec_segment(Trimmer::Trim(63)), 63)]
    #[case(mmap_segment(Trimmer::Trim(63)), 63)]
    #[case(vec_segment(Trimmer::Trim(CAPACITY)), CAPACITY)]
    #[case(mmap_segment(Trimmer::Trim(CAPACITY)), CAPACITY)]
    #[case(vec_segment(Trimmer::Trim(CAPACITY * 2)), CAPACITY * 2)]
    #[case(mmap_segment(Trimmer::Trim(CAPACITY * 2)), CAPACITY * 2)]
    fn test_extend_from_slice_trimmer<S: Storage<Record = usize>>(
        #[case] mut segment: Segment<S>,
        #[case] trim: usize,
    ) {
        // Test records.
        assert!(segment.is_empty());
        assert_eq!(CAPACITY, segment.capacity());

        let records: [_; CAPACITY] = std::array::from_fn(|i| i * 2);
        assert!(segment.extend_from_slice(&records).is_empty());
        assert_eq!(&records, segment.records());

        // More than capacity should be rejected.
        let more_records: [_; CAPACITY] = std::array::from_fn(|i| i * 3);
        let rejected = segment.extend_from_slice(&more_records);

        // Everything more than how many records we trimmed should be rejected.
        let trimmed = std::cmp::min(CAPACITY, trim);
        assert_eq!(rejected, &more_records[trimmed..]);

        // Make sure expected state.
        assert_eq!(0, segment.remaining());
        assert_eq!(segment.len(), CAPACITY);
        assert_eq!(
            &segment.records()[..CAPACITY - trimmed],
            &records[trimmed..]
        );
        assert_eq!(
            &segment.records()[CAPACITY - trimmed..],
            &more_records[..trimmed]
        );
    }
}
