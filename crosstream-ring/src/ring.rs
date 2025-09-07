//! Definition of storage engine that backs a Ring.

use crate::Record;
use memmap2::{MmapMut, MmapOptions};
use std::cmp::min;
use std::{borrow::Borrow, marker::PhantomData, ops::Deref};

#[derive(Debug)]
pub struct Ring<T> {
    len: usize,
    cap: usize,
    memory: MmapMut,
    phantom: PhantomData<T>,
}

impl<T: Record> Ring<T> {
    pub fn with_capacity(capacity: usize) -> Self {
        let mmap = MmapOptions::new()
            .len(capacity * T::size())
            .map_anon()
            .expect("Cannot allocate memory for segment");

        Self {
            cap: capacity,
            len: 0,
            phantom: PhantomData,
            memory: mmap,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    #[inline]
    pub fn remaining(&self) -> usize {
        self.cap - self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.cap == self.len
    }

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

    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl<T: Record> Deref for Ring<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        let end = self.len * T::size();
        T::from_bytes_slice(&self.memory[..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::{check, generator::*};

    /// Maximum capacity of the test ring buffer.
    const RING_CAPACITY: usize = 1024 * 1024;

    /// Different types of ring buffer operations.
    #[derive(Debug, TypeGenerator)]
    enum Operation {
        Clear,
        Trim(u8),
        Push(u64),
        Extend(Vec<u64>),
    }

    /// Methods of a ring buffer being tested.
    trait RingBuffer<T> {
        fn test_clear(&mut self);

        fn test_trim(&mut self, len: &u8);

        fn test_push(&mut self, record: &T);

        fn test_extend_slice(&mut self, records: &[T]);

        fn test_records(&self) -> &[T];
    }

    // Reference implementation of ring buffer using a Vec.
    impl<T: Copy> RingBuffer<T> for Vec<T> {
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
    impl<T: Record + Copy> RingBuffer<T> for Ring<T> {
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

    #[test]
    fn ring_state_machine() {
        check!()
            .with_type::<Vec<Operation>>()
            .for_each(|operations| {
                let mut ring = Ring::with_capacity(RING_CAPACITY);
                let mut vec = Vec::with_capacity(RING_CAPACITY);

                for operation in operations {
                    match operation {
                        Operation::Clear => {
                            ring.test_clear();
                            vec.test_clear();
                        }

                        Operation::Trim(len) => {
                            ring.test_trim(len);
                            vec.test_trim(len);
                        }

                        Operation::Push(record) => {
                            ring.test_push(record);
                            vec.test_push(record);
                        }

                        Operation::Extend(records) => {
                            ring.test_extend_slice(records);
                            vec.test_extend_slice(records);
                        }
                    }

                    assert_eq!(ring.test_records(), vec.test_records());
                }
            })
    }
}
