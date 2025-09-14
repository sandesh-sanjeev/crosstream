//! Definition of fixed size types with compile time known layout, size and alignment.

/// Fixed sized type with compile time known layout, size and alignment.
///
/// The basic idea is that this type provides support for zero-copy transmutation
/// between a record and byte slice. You probably don't want to handwrite these
/// yourself. There are crates that allows one to safely perform this transmutation.
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

// There will be conflicting implementations if both features are enabled.
#[cfg(all(feature = "zerocopy", feature = "bytemuck"))]
compile_error!("Either zerocopy or bytemuck feature can to be enabled, not both");

// Support for zero copy transmutation for compatible types from bytemuck crate.
#[cfg(all(feature = "bytemuck", not(feature = "zerocopy")))]
use bytemuck::{AnyBitPattern, NoUninit, bytes_of, cast_slice, from_bytes, must_cast_slice};

#[cfg(all(feature = "bytemuck", not(feature = "zerocopy")))]
impl<T: AnyBitPattern + NoUninit> Record for T {
    fn size() -> usize {
        size_of::<T>()
    }

    fn to_bytes(record: &Self) -> &[u8] {
        bytes_of(record)
    }

    fn from_bytes(bytes: &[u8]) -> &Self {
        from_bytes(bytes)
    }

    fn to_bytes_slice(records: &[Self]) -> &[u8] {
        must_cast_slice(records)
    }

    fn from_bytes_slice(bytes: &[u8]) -> &[Self] {
        cast_slice(bytes)
    }
}

// Support for zero copy transmutation for compatible types from zerocopy crate.
#[cfg(all(feature = "zerocopy", not(feature = "bytemuck")))]
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[cfg(all(feature = "zerocopy", not(feature = "bytemuck")))]
impl<T: FromBytes + IntoBytes + Immutable + KnownLayout> Record for T {
    fn size() -> usize {
        size_of::<T>()
    }

    fn to_bytes(record: &Self) -> &[u8] {
        record.as_bytes()
    }

    fn from_bytes(bytes: &[u8]) -> &Self {
        T::ref_from_bytes(bytes).expect("Zerocopy type transmutation error")
    }

    fn to_bytes_slice(records: &[Self]) -> &[u8] {
        records.as_bytes()
    }

    fn from_bytes_slice(bytes: &[u8]) -> &[Self] {
        <[T]>::ref_from_bytes(bytes).expect("Zerocopy type transmutation error")
    }
}

#[cfg(test)]
#[cfg(feature = "bytemuck")]
mod tests {
    use super::*;
    use bolero::{TypeGenerator, check};
    use bytemuck::{Pod, Zeroable};

    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, TypeGenerator, Pod, Zeroable)]
    struct Log {
        seq_no: u64,
        offset: usize,
    }

    #[test]
    fn round_trip_record() {
        check!().with_type::<Log>().for_each(|record| {
            // Transmute to bytes
            let bytes = Log::to_bytes(record);
            assert_eq!(Log::size(), bytes.len());

            // Transmute from bytes.
            let returned = Log::from_bytes(bytes);
            assert_eq!(record, returned);
        });
    }

    #[test]
    fn round_trip_record_slice() {
        check!().with_type::<Vec<Log>>().for_each(|records| {
            // Transmute to bytes
            let bytes = Log::to_bytes_slice(records);
            assert_eq!(Log::size() * records.len(), bytes.len());

            // Transmute from bytes.
            let returned = Log::from_bytes_slice(bytes);
            assert_eq!(records, returned);
        });
    }
}

#[cfg(test)]
#[cfg(feature = "zerocopy")]
mod tests {
    use super::*;
    use bolero::{TypeGenerator, check};

    #[repr(C)]
    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        TypeGenerator,
        IntoBytes,
        FromBytes,
        KnownLayout,
        Immutable,
    )]
    struct Log {
        seq_no: u64,
        data: [u8; 16],
    }

    #[test]
    fn round_trip_record() {
        check!().with_type::<Log>().for_each(|record| {
            // Transmute to bytes
            let bytes = Log::to_bytes(record);
            assert_eq!(Log::size(), bytes.len());

            // Transmute from bytes.
            let returned = Log::from_bytes(bytes);
            assert_eq!(record, returned);
        });
    }

    #[test]
    fn round_trip_record_slice() {
        check!().with_type::<Vec<Log>>().for_each(|records| {
            // Transmute to bytes
            let bytes = Log::to_bytes_slice(records);
            assert_eq!(Log::size() * records.len(), bytes.len());

            // Transmute from bytes.
            let returned = Log::from_bytes_slice(bytes);
            assert_eq!(records, returned);
        });
    }
}
