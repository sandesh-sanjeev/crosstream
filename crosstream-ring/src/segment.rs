//! Definition of a container of contiguous elements.

use crate::{
    MemStorage, OffHeap, OffHeapStorage, OnHeap, OnHeapStorage, Record, Storage, VecStorage,
};
use std::cmp::min;

/// Type alias for a [`Segment`] backed by [`VecStorage`].
pub type VecSegment<T> = Segment<VecStorage<T>>;

/// Type alias for a [`Segment`] backed by [`OffHeapStorage`].
pub type OffHeapSegment<T> = Segment<OffHeapStorage<T>>;

/// Type alias for a [`Segment`] backed by [`OnHeapStorage`].
pub type OnHeapSegment<T> = Segment<OnHeapStorage<T>>;

/// Segment is a container of contiguous elements.
///
/// The intended purpose for this is as a building block for a high performance off-heap
/// ring buffer. But might be useful in other cases that requires large amounts of data
/// in memory (maybe on disk on day).
///
/// * Does not support growth, i.e, cannot be resized to increase capacity.
/// * There can be no gaps between elements, push/remove from back, but only remove from front.
/// * As of now (and probably forever) only supports elements that implement [`Record`].
/// * Performance of Segment operations is virtually identical to that of [`Vec`].
/// * Supports few different types of storage engines; [`VecSegment`], [`OffHeapSegment`] and [`OnHeapSegment`].
#[derive(Debug)]
pub struct Segment<S: Storage> {
    storage: S,
    trimmer: Trimmer,
}

impl<T: Record + Copy> VecSegment<T> {
    /// Create a new instance of Segment using `Vec` for memory.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this segment can accommodate.
    /// * `trimmer` - Trimmer to use when appending records into segment.
    pub fn with_capacity(capacity: usize, trimmer: Trimmer) -> VecSegment<T> {
        Self {
            trimmer,
            storage: VecStorage::new(capacity),
        }
    }
}

impl<T: Record> OffHeapSegment<T> {
    /// Create a new instance of Segment using memory allocated off-heap.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this segment can accommodate.
    /// * `trimmer` - Trimmer to use when appending records into segment.
    pub fn with_capacity(capacity: usize, trimmer: Trimmer) -> OffHeapSegment<T> {
        Self {
            trimmer,
            storage: MemStorage::<_, OffHeap>::new(capacity),
        }
    }
}

impl<T: Record> OnHeapSegment<T> {
    /// Create a new instance of Segment using memory allocated on heap.
    ///
    /// * TODO: Add support for huge pages.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of elements this segment can accommodate.
    /// * `trimmer` - Trimmer to use when appending records into segment.
    pub fn with_capacity(capacity: usize, trimmer: Trimmer) -> OnHeapSegment<T> {
        Self {
            trimmer,
            storage: MemStorage::<_, OnHeap>::new(capacity),
        }
    }
}

impl<S: Storage> Segment<S> {
    /// Number of records currently stored in this Segment.
    pub fn len(&self) -> usize {
        self.storage.length()
    }

    /// Maximum number of records that can be stored in this Segment.
    pub fn capacity(&self) -> usize {
        self.storage.capacity()
    }

    /// Number of records that can be appended to this Segment without overflow.
    pub fn remaining(&self) -> usize {
        self.storage.remaining()
    }

    /// true if this Segment has no records, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// true if this Segment is at capacity, false otherwise.
    pub fn is_full(&self) -> bool {
        self.capacity() == self.len()
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
        if len >= self.len() {
            self.clear();
            return;
        }

        // We need to left shift some bytes.
        self.storage.trim(len);
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
        self.storage.extend(&[record]);

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
        self.storage.extend(to_append);

        // Return all the rejected records.
        to_reject
    }

    /// Remove all elements from this Segment.
    ///
    /// * This is a constant time O(1) operation.
    pub fn clear(&mut self) {
        self.storage.clear();
    }

    /// Returns reference to all the records in a segment.
    pub fn records(&self) -> &[S::Record] {
        self.storage.records()
    }

    fn run_trimmer(&mut self) {
        let trim_len = match self.trimmer {
            Trimmer::None => 0,
            Trimmer::Trim(len) => len,
        };

        if trim_len > 0 {
            self.trim(trim_len);
        }
    }
}

/// Strategy used to trim records during appends into a [`Segment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(bolero::TypeGenerator))]
pub enum Trimmer {
    /// When an append operation occurs and there isn't sufficient capacity
    /// to accommodate records, this does nothing. Meaning one or more records
    /// might be rejected from the segment.
    None,

    /// When an append operation occurs and there isn't sufficient capacity
    /// to accommodate records, this trims first N records from the segment.
    /// However records will  be rejected if N < number of records being appended.
    Trim(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    const CAPACITY: usize = 1024;

    fn on_heap_segment(trimmer: Trimmer) -> OnHeapSegment<usize> {
        OnHeapSegment::with_capacity(CAPACITY, trimmer)
    }

    fn off_heap_segment(trimmer: Trimmer) -> OffHeapSegment<usize> {
        OffHeapSegment::with_capacity(CAPACITY, trimmer)
    }

    fn vec_segment(trimmer: Trimmer) -> VecSegment<usize> {
        VecSegment::with_capacity(CAPACITY, trimmer)
    }

    #[rstest]
    #[case(vec_segment(Trimmer::None))]
    #[case(off_heap_segment(Trimmer::None))]
    #[case(on_heap_segment(Trimmer::None))]
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
    #[case(off_heap_segment(Trimmer::Trim(100)), 100)]
    #[case(on_heap_segment(Trimmer::Trim(100)), 100)]
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
    #[case(vec_segment(Trimmer::None))]
    #[case(off_heap_segment(Trimmer::None))]
    #[case(on_heap_segment(Trimmer::None))]
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
    #[case(off_heap_segment(Trimmer::Trim(63)), 63)]
    #[case(on_heap_segment(Trimmer::Trim(63)), 63)]
    #[case(vec_segment(Trimmer::Trim(CAPACITY)), CAPACITY)]
    #[case(off_heap_segment(Trimmer::Trim(CAPACITY)), CAPACITY)]
    #[case(on_heap_segment(Trimmer::Trim(CAPACITY)), CAPACITY)]
    #[case(vec_segment(Trimmer::Trim(CAPACITY * 2)), CAPACITY * 2)]
    #[case(off_heap_segment(Trimmer::Trim(CAPACITY * 2)), CAPACITY * 2)]
    #[case(on_heap_segment(Trimmer::Trim(CAPACITY * 2)), CAPACITY * 2)]
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
