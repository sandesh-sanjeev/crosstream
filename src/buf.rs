//! Definition of reusable buffer that can hold sequenced records.

use crate::{OnHeapStorage, SeqRecord, Storage};

/// A reusable buffer to query from ring buffers.
#[derive(Debug)]
pub struct QueryBuf<T>(OnHeapStorage<T>);

impl<T: SeqRecord> QueryBuf<T> {
    /// Create a new instance of [`QueryBuf`].
    ///
    /// # Panic
    ///
    /// * Panics if capacity == 0.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of records to hold in buffer.
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "Buf should have capacity > 0");
        Self(OnHeapStorage::new(capacity))
    }

    /// Number of records currently held in buffer.
    pub fn length(&self) -> usize {
        self.0.length()
    }

    /// Maximum number of records buffer can hold.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Slice of records held in the buffer.
    pub fn records(&self) -> &[T] {
        self.0.records()
    }

    /// Number of records that can be appended without overflow.
    #[allow(dead_code)]
    pub(crate) fn remaining(&self) -> usize {
        self.0.remaining()
    }

    /// Append new records into buffer.
    ///
    /// # Invariants
    ///
    /// * records.len() <= self.remaining()
    /// * records.len() > 0
    ///
    /// # Arguments
    ///
    /// * `records` - Records to append.
    #[allow(dead_code)]
    pub(crate) fn extend(&mut self, records: &[T]) {
        self.0.extend(records);
    }

    /// Clear all records from [`Buf`].
    #[allow(dead_code)]
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }
}

#[cfg(test)]
#[cfg(any(feature = "zerocopy", feature = "bytemuck"))]
mod tests {
    use super::*;

    #[cfg(feature = "bytemuck")]
    use bytemuck::{Pod, Zeroable};

    #[cfg(feature = "zerocopy")]
    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

    const CAPACITY: usize = 1024;

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

    #[test]
    fn state_machine() {
        let mut buf = QueryBuf::new(CAPACITY);
        assert_eq!(buf.capacity(), CAPACITY);

        let records: Vec<_> = (1..=CAPACITY as u64).map(Log).collect();
        for batch_size in 1..=CAPACITY {
            // Clear state for new test run.
            buf.clear();

            // Assert starting state.
            assert_eq!(buf.length(), 0);
            assert_eq!(buf.remaining(), CAPACITY);
            assert_eq!(buf.records(), &[]);

            // Append records till buffer is filled.
            let mut len = 0;
            for chunk in records.chunks(batch_size) {
                assert!(buf.remaining() > 0);
                buf.extend(chunk);
                len += chunk.len();

                // Assert current state.
                assert_eq!(buf.length(), len);
                assert_eq!(buf.remaining(), CAPACITY - len);
                assert_eq!(buf.records(), &records[..len])
            }

            // Assert final state.
            assert_eq!(buf.remaining(), 0);
            assert_eq!(buf.length(), CAPACITY);
            assert_eq!(buf.records(), &records);
        }
    }

    #[test]
    #[should_panic]
    fn zero_capacity_panic() {
        QueryBuf::<Log>::new(0);
    }
}
