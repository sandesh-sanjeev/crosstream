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

#[cfg(kani)]
mod verification {
    use super::*;
    use pastey::paste;

    macro_rules! round_trip_proof {
        ($($type:ty),*) => {
            paste! {
                $(
                    #[kani::proof]
                    fn [<round_trip_ $type _proof>]() {
                        let record: $type = kani::any();

                        let bytes = $type::to_bytes(&record);
                        assert_eq!($type::size(), bytes.len());

                        let returned = $type::from_bytes(bytes);
                        assert_eq!(&record, returned);
                    }

                    #[kani::proof]
                    #[kani::unwind(256)]
                    fn [<round_trip_ $type _slice_proof>]() {
                        let records: Vec<$type> = kani::bounded_any::<_, 8>();

                        let bytes = $type::to_bytes_slice(&records);
                        assert_eq!($type::size() * records.len(), bytes.len());

                        let returned = $type::from_bytes_slice(bytes);
                        assert_eq!(&records, returned);
                    }

                    #[kani::proof]
                    fn [<round_trip_ $type _array_proof>]() {
                        let record: [$type; 3] = kani::any();

                        let bytes = <[$type; 3]>::to_bytes(&record);
                        assert_eq!(<[$type; 3]>::size(), bytes.len());

                        let returned = <[$type; 3]>::from_bytes(bytes);
                        assert_eq!(&record, returned);
                    }

                    #[kani::proof]
                    #[kani::unwind(256)]
                    fn [<round_trip_ $type _array_slice_proof>]() {
                        let records: Vec<[$type; 3]> = kani::bounded_any::<_, 4>();

                        let bytes = <[$type; 3]>::to_bytes_slice(&records);
                        assert_eq!(<[$type; 3]>::size() * records.len(), bytes.len());

                        let returned = <[$type; 3]>::from_bytes_slice(bytes);
                        assert_eq!(&records, returned);
                    }
                )*
            }
        };
    }

    round_trip_proof!(
        u8, u16, u32, u64, usize, u128, // Unsigned integers
        i8, i16, i32, i64, isize, i128 // Signed integers
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::check;
    use pastey::paste;

    macro_rules! round_trip_test {
        ($($type:ty),*) => {
            paste! {
                $(
                    #[test]
                    fn [<round_trip_record_ $type _test>]() {
                        check!()
                            .with_iterations(100)
                            .with_type::<$type>()
                            .for_each(|record| {
                                let bytes = <$type>::to_bytes(record);
                                assert_eq!(<$type>::size(), bytes.len());

                                let returned = <$type>::from_bytes(bytes);
                                assert_eq!(record, returned);
                            })
                    }

                    #[test]
                    fn [<round_trip_record_slice_ $type _test>]() {
                        check!()
                            .with_iterations(100)
                            .with_type::<Vec<$type>>()
                            .for_each(|records| {
                                let bytes = <$type>::to_bytes_slice(records);
                                assert_eq!(<$type>::size() * records.len(), bytes.len());

                                let returned = <$type>::from_bytes_slice(bytes);
                                assert_eq!(records, returned);
                            })
                    }

                    #[test]
                    fn [<round_trip_record_array_3_ $type _test>]() {
                        check!()
                            .with_iterations(100)
                            .with_type::<[$type; 3]>()
                            .for_each(|record| {
                                let bytes = <[$type; 3]>::to_bytes(record);
                                assert_eq!(<[$type; 3]>::size(), bytes.len());

                                let returned = <[$type; 3]>::from_bytes(bytes);
                                assert_eq!(record, returned);
                            });
                    }

                    #[test]
                    fn [<round_trip_record_array_3_slice_ $type _test>]() {
                        check!()
                            .with_iterations(100)
                            .with_type::<Vec<[$type; 3]>>()
                            .for_each(|records| {
                                let bytes = <[$type; 3]>::to_bytes_slice(records);
                                assert_eq!(<[$type; 3]>::size() * records.len(), bytes.len());

                                let returned = <[$type; 3]>::from_bytes_slice(bytes);
                                assert_eq!(records, returned);
                            });
                    }
                )*
            }
        };
    }

    // TODO: f32 and f64 do not implement Eq
    round_trip_test!(
        u8, u16, u32, u64, usize, u128, // Unsigned integers
        i8, i16, i32, i64, isize, i128 // Signed integers
    );
}
