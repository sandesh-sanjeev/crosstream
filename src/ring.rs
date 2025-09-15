//! Definition of a ring buffer.

use crate::{OffHeapStorage, OnHeapStorage, QueryBuf, SeqRecord, Storage, VecStorage};
use std::collections::BTreeMap;
use thiserror::Error;

/// Type alias for a [`SeqRing`] backed by [`VecStorage`].
pub type VecSeqRing<T> = SeqRing<VecStorage<T>>;

/// Type alias for a [`SeqRing`] backed by [`OnHeapStorage`].
pub type OnHeapSeqRing<T> = SeqRing<OnHeapStorage<T>>;

/// Type alias for a [`SeqRing`] backed by [`OffHeapStorage`].
pub type OffHeapSeqRing<T> = SeqRing<OffHeapStorage<T>>;

/// Different types of error that can happen when records are appended to [`SeqRing`].
#[derive(Debug, Error)]
pub enum AppendError {
    #[error("Records appended out of sequence. Prev: {0}, Record: {1}")]
    Sequence(u64, u64),
}

/// An in-memory Ring buffer that holds [`SeqRecord`]s.
///
/// Works pretty much like any other ring buffer, few differences:
/// * Performs strict sequence validations against appended records.
/// * Can query from logical positions in ring buffer or via record sequence numbers.
#[derive(Debug)]
pub struct SeqRing<T> {
    prev_seq_no: u64,
    free_slots: Vec<T>,
    slots: BTreeMap<u64, T>,
}

impl<R: SeqRecord + Copy> VecSeqRing<R> {
    /// Create a new ring buffer using a Vec for storage.
    ///
    /// # Panic
    ///
    /// * Panics if number of slots is <= 1.
    /// * Panics if slot_capacity is < 1.
    ///
    /// # Arguments
    ///
    /// * `slot_capacity` - Capacity of an individual slot.
    /// * `slots` - Number of slots in the ring buffer.
    /// * `prev_seq_no` - Sequence number of previous record in sequence.
    pub fn new(slot_capacity: usize, slots: usize, prev_seq_no: u64) -> Self {
        assert!(slots > 1, "A Ring must have at least 2 slots");
        assert!(slot_capacity > 0, "A Ring slot must hold at least 1 record");

        // Allocate all memory and
        let free_slots = std::iter::repeat_with(|| VecStorage::new(slot_capacity))
            .take(slots)
            .collect();

        // Build and return a new Ring.
        SeqRing::from_parts(free_slots, prev_seq_no)
    }
}

impl<R: SeqRecord> OnHeapSeqRing<R> {
    /// Create a new ring buffer using on-heap memory for storage.
    ///
    /// # Panic
    ///
    /// * Panics if number of slots is <= 1.
    /// * Panics if slot_capacity is < 1.
    ///
    /// # Arguments
    ///
    /// * `slot_capacity` - Capacity of an individual slot.
    /// * `slots` - Number of slots in the ring buffer.
    /// * `prev_seq_no` - Sequence number of previous record in sequence.
    pub fn new(slot_capacity: usize, slots: usize, prev_seq_no: u64) -> Self {
        assert!(slots > 1, "A Ring must have at least 2 slots");
        assert!(slot_capacity > 0, "A Ring slot must hold at least 1 record");

        // Allocate all memory and
        let free_slots = std::iter::repeat_with(|| OnHeapStorage::new(slot_capacity))
            .take(slots)
            .collect();

        // Build and return a new Ring.
        SeqRing::from_parts(free_slots, prev_seq_no)
    }
}

impl<R: SeqRecord> OffHeapSeqRing<R> {
    /// Create a new ring buffer using off-heap memory for storage.
    ///
    /// # Panic
    ///
    /// * Panics if number of slots is <= 1.
    /// * Panics if slot_capacity is < 1.
    ///
    /// # Arguments
    ///
    /// * `slot_capacity` - Capacity of an individual slot.
    /// * `slots` - Number of slots in the ring buffer.
    /// * `prev_seq_no` - Sequence number of previous record in sequence.
    pub fn new(slot_capacity: usize, slots: usize, prev_seq_no: u64) -> Self {
        assert!(slots > 1, "A Ring must have at least 2 slots");
        assert!(slot_capacity > 0, "A Ring slot must hold at least 1 record");

        // Allocate all memory and
        let free_slots = std::iter::repeat_with(|| OffHeapStorage::new(slot_capacity))
            .take(slots)
            .collect();

        // Build and return a new Ring.
        SeqRing::from_parts(free_slots, prev_seq_no)
    }
}

impl<R: SeqRecord, T: Storage<Record = R>> SeqRing<T> {
    /// Construct a ring buffer from it's basic parts.
    ///
    /// # Arguments
    ///
    /// * `free_slots` - Uninitialized storage memory.
    /// * `prev_seq_no` - Sequence number of the previous record in sequence.
    fn from_parts(mut free_slots: Vec<T>, prev_seq_no: u64) -> Self {
        // Initialize latest slot in the ring buffer.
        let storage = free_slots.pop().expect("Ring has > 1 slots");
        let mut slots = BTreeMap::new();
        slots.insert(prev_seq_no, storage);

        Self {
            slots,
            free_slots,
            prev_seq_no,
        }
    }

    /// Append new records into the ring buffer.
    ///
    /// If records are added out of order, i.e, sequence number of a record is
    /// <= sequence number of the previous records, an [`AppendError`] is returned.
    ///
    /// # Arguments
    ///
    /// * `records` - Records to append.
    pub fn append(&mut self, mut records: &[R]) -> Result<(), AppendError> {
        // Early return if there are no records to append.
        let Some(record) = records.first() else {
            return Ok(());
        };

        // Sequence validation for records being appended.
        // Sequence of other records are checked when creating buffer.
        if record.seq_no() <= self.prev_seq_no {
            return Err(AppendError::Sequence(self.prev_seq_no, record.seq_no()));
        }

        // Append records into slots till we have consumed all records.
        while !records.is_empty() {
            // Get the latest slot in the ring buffer.
            let mut entry = self.slots.last_entry().expect("Ring has > 1 slots");

            // Check how many records can be appended into the slot.
            let slot = entry.get_mut();
            let (append, next_append) = unsafe {
                // Safety: We just made sure remaining is < records.len().
                let mid = std::cmp::min(slot.remaining(), records.len());
                records.split_at_unchecked(mid)
            };

            // Remaining records for next iteration.
            records = next_append;

            // If the slot has some space, add those records.
            if !append.is_empty() {
                // Write records into the slot.
                slot.extend(append);

                // Update seq_no from accepted records.
                self.prev_seq_no = append
                    .last()
                    .map(SeqRecord::seq_no)
                    .expect("Non empty records should have last");
            }

            // Prepare for next iteration.
            if !records.is_empty() {
                // Create a new slot for new records.
                // If there is unused storage, we start using that.
                let mut new_slot = self.free_slots.pop().unwrap_or_else(|| {
                    // No storage in free list, reclaim space from oldest slot.
                    self.slots
                        .pop_first()
                        .map(|(_, storage)| storage)
                        .expect("Ring has > 1 slot")
                });

                // Add the new slot into the ring buffer.
                new_slot.clear(); // Clear any accumulated state in storage.
                self.slots.insert(self.prev_seq_no, new_slot);
            }
        }

        Ok(()) // Records appended successfully.
    }

    /// Query for records from the beginning.
    ///
    /// * Records returned are sorted in ascending order of their sequence numbers.
    /// * buf is cleared of any existing records to make space for records from query.
    /// * buf is filled to capacity, if ring buffer has enough records.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to copy records into.
    pub fn query_from_trim(&self, buf: &mut QueryBuf<R>) {
        // Clear records to make space for the new query.
        buf.clear();

        // Fill buffer till full.
        for (_, slot) in self.slots.range(..) {
            // We've collected enough records.
            if buf.remaining() == 0 {
                break;
            }

            // Figure out range of records to copy.
            let records = slot.records();
            let (copy, _) = unsafe {
                let mid = std::cmp::min(buf.remaining(), records.len());
                records.split_at_unchecked(mid)
            };

            // Copy the range of records into buffer.
            buf.extend(copy);
        }
    }

    /// Query for records from after a specific sequence number.
    ///
    /// * Records returned are sorted in ascending order of their sequence numbers.
    /// * buf is cleared of any existing records to make space for records from query.
    /// * buf is filled to capacity, if ring buffer has enough records.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to copy records into.
    pub fn query_after(&self, seq_no: u64, buf: &mut QueryBuf<R>) {
        // Clear records to make space for the new query.
        buf.clear();

        // Slots are stored something like this.
        //
        // 3 -> [4, 5, 6]
        // 6 -> [7, 8, 9]
        //
        // With range(..seq_no):
        // After 0 = None
        // After 1 = None
        // After 2 = None
        // After 3 = Some((3, [4, 5, 6]))
        // After 4 = Some((3, [4, 5, 6]))
        // After 5 = Some((3, [4, 5, 6]))
        // After 6 = Some((6, [7, 8, 9]))
        // After 7 = Some((6, [7, 8, 9]))
        // After 8 = Some((6, [7, 8, 9]))
        // After 9 = Some((6, [7, 8, 9]))
        // After 10 = Some((6, [7, 8, 9]))
        // After 11 = Some((6, [7, 8, 9]))
        // After 12 = Some((6, [7, 8, 9]))

        // Return early if seq_no is not yet appended.
        if self.prev_seq_no <= seq_no {
            return;
        }

        // Fetch the starting slot to begin iteration.
        let (start_seq_no, _) = self
            .slots
            .range(..=seq_no)
            .next_back()
            .unwrap_or_else(|| self.slots.first_key_value().expect("Ring has > 1 slot"));

        // Fill buffer till full.
        for (page_seq_no, slot) in self.slots.range(start_seq_no..) {
            // We've collected enough records.
            if buf.remaining() == 0 {
                break;
            }

            // Fetch match records from the slot.
            let records = if page_seq_no == start_seq_no {
                let records = slot.records();
                let index = match records.binary_search_by_key(&seq_no, SeqRecord::seq_no) {
                    // Means record was not found, but would have been in this index.
                    // So we can start from this index.
                    Err(index) => index,

                    // Means record is found in this index, so we want to start from next index.
                    // Can never overflow because max size of non ZST array is isize
                    Ok(index) => index + 1,
                };

                // Return all records from the given index.
                // Safety: We return early if searching for seq_no that has not yet been appended.
                unsafe { records.split_at_unchecked(index).1 }
            } else {
                // If we get to this point, means that we consumed all records from previous
                // slot. So, we can start from the very beginning of the slot.
                slot.records()
            };

            // Figure out range of records to copy.
            let (copy, _) = unsafe {
                let mid = std::cmp::min(buf.remaining(), records.len());
                records.split_at_unchecked(mid)
            };

            // Copy the range of records into buffer.
            buf.extend(copy);
        }
    }
}

#[cfg(test)]
#[cfg(any(feature = "zerocopy", feature = "bytemuck"))]
mod tests {
    use super::*;
    use rstest::rstest;

    #[cfg(feature = "bytemuck")]
    use bytemuck::{Pod, Zeroable};

    #[cfg(feature = "zerocopy")]
    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

    const SLOTS: usize = 3;
    const SLOT_CAPACITY: usize = 1024;
    const MAX_CAPACITY: usize = SLOTS * SLOT_CAPACITY;

    #[cfg(feature = "zerocopy")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
    struct Log(u64);

    #[cfg(feature = "bytemuck")]
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
    struct Log(u64);

    impl SeqRecord for Log {
        fn seq_no(&self) -> u64 {
            self.0
        }
    }

    fn vec_ring(prev_seq_no: u64) -> VecSeqRing<Log> {
        VecSeqRing::new(SLOT_CAPACITY, SLOTS, prev_seq_no)
    }

    fn on_heap_ring(prev_seq_no: u64) -> OnHeapSeqRing<Log> {
        OnHeapSeqRing::new(SLOT_CAPACITY, SLOTS, prev_seq_no)
    }

    fn off_heap_ring(prev_seq_no: u64) -> OffHeapSeqRing<Log> {
        OffHeapSeqRing::new(SLOT_CAPACITY, SLOTS, prev_seq_no)
    }

    #[rstest]
    #[case(vec_ring(0))]
    #[case(on_heap_ring(0))]
    #[case(off_heap_ring(0))]
    fn append<S: Storage<Record = Log>>(#[case] mut ring: SeqRing<S>) -> Result<(), AppendError> {
        let mut prev_seq_no = 0;

        // Append into buffer in different batch sizes.
        for batch_size in (256..=MAX_CAPACITY).step_by(256) {
            // Test records appended into the ring buffer.
            let start_seq_no = prev_seq_no + 1;
            let end_seq_no = start_seq_no + MAX_CAPACITY as u64;
            let records: Vec<_> = (start_seq_no..=end_seq_no).map(Log).collect();

            // Append to ring in chunks.
            for chunk in records.chunks(batch_size) {
                ring.append(chunk)?;

                if let Some(log) = chunk.last() {
                    prev_seq_no = log.seq_no();
                }
            }
        }

        // Make sure expected state.
        let mut buf = QueryBuf::new(MAX_CAPACITY);
        ring.query_from_trim(&mut buf);
        let ring_records: Vec<_> = buf.records().iter().map(|log| *log).collect();

        // Last record must be the previous record appended.
        if let Some(last) = ring_records.last() {
            assert_eq!(last.seq_no(), prev_seq_no);
        }

        // Every record must be greater than the previous one.
        for pair in ring_records.windows(2) {
            assert!(pair[1].seq_no() > pair[0].seq_no());
        }

        // Empty records should be no-op.
        ring.append(&[])?;

        // Append out of order should return error.
        match ring.append(&[Log(1)]) {
            Ok(_) => panic!("Unexpected success!"),
            Err(AppendError::Sequence(prev, record)) => {
                assert_eq!(prev, prev_seq_no);
                assert_eq!(record, 1)
            }
        }

        // Cannot append latest seq_no either.
        match ring.append(&[Log(prev_seq_no)]) {
            Ok(_) => panic!("Unexpected success!"),
            Err(AppendError::Sequence(prev, record)) => {
                assert_eq!(prev, prev_seq_no);
                assert_eq!(record, prev_seq_no)
            }
        }

        // Ring buffer should not have changed.
        ring.query_from_trim(&mut buf);
        assert_eq!(buf.records(), &ring_records);

        Ok(())
    }

    #[rstest]
    #[case(vec_ring(0))]
    #[case(on_heap_ring(0))]
    #[case(off_heap_ring(0))]
    fn query_from_trim<S: Storage<Record = Log>>(
        #[case] mut ring: SeqRing<S>,
    ) -> Result<(), AppendError> {
        // Test records appended into the ring buffer.
        let records: Vec<_> = (1..=MAX_CAPACITY as u64).map(Log).collect();

        // Append all the test records into ring buffer.
        ring.append(&records)?;

        // Query for records with different batch sizes.
        for batch_size in (256..=MAX_CAPACITY).step_by(256) {
            // Buffer to query for records from the ring.
            let mut buf = QueryBuf::new(batch_size);

            // Query from the very beginning.
            ring.query_from_trim(&mut buf);

            // Make sure expected records were returned.
            assert_eq!(&records[..batch_size], buf.records());
        }
        Ok(())
    }

    #[rstest]
    #[case(vec_ring(0), 1)]
    #[case(on_heap_ring(0), 1)]
    #[case(off_heap_ring(0), 1)]
    #[case(vec_ring(0), 2)]
    #[case(on_heap_ring(0), 2)]
    #[case(off_heap_ring(0), 2)]
    #[case(vec_ring(0), 3)]
    #[case(on_heap_ring(0), 3)]
    #[case(off_heap_ring(0), 3)]
    fn query_after<S: Storage<Record = Log>>(
        #[case] mut ring: SeqRing<S>,
        #[case] skip_size: usize,
    ) -> Result<(), AppendError> {
        // Test records appended into the ring buffer.
        let records: Vec<_> = (1..=MAX_CAPACITY as u64)
            .map(|seq_no| seq_no * skip_size as u64)
            .map(Log)
            .collect();

        // Append all the test records into ring buffer.
        ring.append(&records)?;

        // Query for records with different batch sizes.
        for batch_size in (256..=MAX_CAPACITY).step_by(256) {
            // Buffer to query for records from the ring.
            let mut buf = QueryBuf::new(batch_size);

            // Query from every sequence number.
            for seq_no in 0..(MAX_CAPACITY * skip_size) {
                ring.query_after(seq_no as _, &mut buf);

                // Make sure expected records were returned.
                let start = seq_no / skip_size;
                let end = std::cmp::min(start + batch_size, records.len());
                assert_eq!(&records[start..end], buf.records());
            }

            // After should return empty results.
            ring.query_after((MAX_CAPACITY * skip_size) as _, &mut buf);
            assert_eq!(buf.records(), &[]);
        }

        Ok(())
    }
}
