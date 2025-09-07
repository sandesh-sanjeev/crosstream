//! Definition of `Ring` compatible elements.

use bytemuck::{bytes_of, cast_slice, from_bytes, must_cast_slice};

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

macro_rules! impl_num_record {
    ($($type:ty),*) => {
        $(
            impl Record for $type {
                #[inline]
                fn size() -> usize {
                    size_of::<$type>()
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
        )*
    };
}

impl_num_record!(
    u8, u16, u32, u64, u128, usize, // Unsigned integers
    i8, i16, i32, i64, i128, isize, // Signed integers
    f32, f64 // Floating point numbers
);
