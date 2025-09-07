//! Definition of `Segment` compatible elements.

use bytemuck::{AnyBitPattern, NoUninit, bytes_of, cast_slice, from_bytes, must_cast_slice};

/// A fixed sized record with compile time known layout, size and alignment.
pub trait Record: Sized {
    /// Size of the record.
    fn size() -> usize;

    /// Zero copy transmute from record to bytes.
    ///
    /// # Arguments
    ///
    /// * `record` - Record to transmute.
    fn to_bytes(record: &Self) -> &[u8];

    /// Zero copy transmute from bytes to record.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Bytes to transmute.
    fn from_bytes(bytes: &[u8]) -> &Self;

    /// Zero copy transmute from record slice to bytes.
    ///
    /// # Arguments
    ///
    /// * `records` - Record slice to transmute.
    fn to_bytes_slice(records: &[Self]) -> &[u8];

    /// Zero copy transmute from bytes to record slice.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Bytes to transmute.
    fn from_bytes_slice(bytes: &[u8]) -> &[Self];
}

impl<T: AnyBitPattern + NoUninit> Record for T {
    #[inline]
    fn size() -> usize {
        size_of::<T>()
    }

    #[inline]
    fn to_bytes(record: &Self) -> &[u8] {
        bytes_of(record)
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> &Self {
        from_bytes(bytes)
    }

    #[inline]
    fn to_bytes_slice(records: &[Self]) -> &[u8] {
        must_cast_slice(records)
    }

    #[inline]
    fn from_bytes_slice(bytes: &[u8]) -> &[Self] {
        cast_slice(bytes)
    }
}
