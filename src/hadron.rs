//! Definition of a ring buffer.

use crate::{Heap, Memory};
use std::marker::PhantomData;

/// Fixed sized type with compile time known layout, size and alignment.
///
/// The basic idea is that this type provides support for zero-copy transmutation
/// between a record and byte slice. You probably don't want to handwrite these
/// yourself because it's quite error prone. There are crates that allows one to
/// safely perform this transmutation.
///
/// * [`zerocopy`](https://docs.rs/zerocopy/latest/zerocopy/)
/// * [`bytemuck`](https://docs.rs/bytemuck/latest/bytemuck/)
pub trait Item: Sized + Copy {
    /// Number of bytes needed to represent this type.
    const SIZE: usize;

    /// Transmute a slice of items to bytes.
    ///
    /// # Arguments
    ///
    /// * `items` - Slice of items to transmute.
    fn to_bytes_slice(items: &[Self]) -> &[u8];

    /// Transmute a slice of bytes into a slice of items.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Bytes to transmute to slice of items.
    fn from_byte_slice(bytes: &[u8]) -> &[Self];
}

/// Hadron is a fixed size ring buffer.
///
/// It is designed for high performance use cases and makes trade-offs to achieve it.
/// Bulk append is guaranteed to be exactly 2 memcpy operations. It provides a reference
/// to items stored in the ring buffer in constant time. The big trade-off here is that
/// only elements of type [`Item`] can be appended into the ring buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hadron<T, Alloc = Heap> {
    // Index where the next append will occur.
    // This will wrap around to 0 when next == cap.
    next: usize,

    // Number of records currently held in the ring buffer.
    length: usize,

    // Maximum number of records this ring buffer can hold.
    capacity: usize,

    // A pre-allocated memory for ring buffer records.
    memory: Alloc,

    // Type of record held in the ring buffer.
    phantom: PhantomData<T>,
}

impl<T: Item> Hadron<T> {
    /// Create a new instance of this ring buffer.
    ///
    /// All required memory is allocated during initialization. It is
    /// guaranteed that no allocations happen after initialization.
    ///
    /// # Panic
    ///
    /// * Ring buffer must have at least one item.
    /// * Number of items in bytes should be <= isize::MAX.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items this ring buffer can hold.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "Hadron must contain at least one item");

        Self {
            capacity,
            next: 0,
            length: 0,
            phantom: PhantomData,
            memory: Heap::alloc::<T>(capacity),
        }
    }
}

impl<T: Item, Alloc: Memory> Hadron<T, Alloc> {
    /// Append items into this ring buffer.
    ///
    /// If newly appended records exceeds the capacity of this ring buffer,
    /// space is reclaimed by evicting old records from the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `items` - Items to append into this ring buffer.
    pub fn append_from_slice(&mut self, mut items: &[T]) {
        // If number of items is greater than the capacity of this ring buffer, some of the items
        // will be overwritten. We can optimize this by skipping those items. This also allows us
        // to make this append at exactly 2 memcpy operations.
        if items.len() > self.capacity {
            let split = items.len() - self.capacity;
            items = items.split_at(split).1;
        }

        // When we reach the end of the ring buffer, we wrap around and overwrite oldest items.
        // Which means we need exactly 2 memcpy operations. One from current index till end of
        // the buffer. Another one to start write from index of 0.
        let remaining = self.capacity - self.next;
        let (first, second) = match items.split_at_checked(remaining) {
            Some(split) => split,
            None => (items, Default::default()),
        };

        // Split the backing memory into discrete writeable chunks.
        let mid = self.next * T::SIZE;
        let (tail, head) = self.memory.as_mut().split_at_mut(mid);

        // Write the first batch.
        let head_bytes = T::to_bytes_slice(first);
        head[..head_bytes.len()].copy_from_slice(head_bytes);

        // Write the second batch.
        let tail_bytes = T::to_bytes_slice(second);
        tail[..tail_bytes.len()].copy_from_slice(tail_bytes);

        // Update state.
        self.next = (self.next + items.len()) % self.capacity;
        self.length = std::cmp::min(self.length + items.len(), self.capacity);
    }

    /// Get a reference to items currently stored in the ring buffer.
    ///
    /// Since the ring buffer can wrap around, items in the ring buffer are stored
    /// in two non-overlapping discrete chunks of items. When the ring buffer is not
    /// full, tail is always empty.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        // If the ring buffer has not wrapped around, then the starting index is always 0.
        let memory = self.memory.as_ref();
        if self.length < self.capacity {
            // There is only head in this case, no tail.
            // Because ring buffer has not wrapped around 0 yet, it's just a single slice.
            let head = &memory[..self.length * T::SIZE];
            (T::from_byte_slice(head), Default::default())
        } else {
            // If the ring buffer has indeed wrapped around, then starting index is the same as next.
            // In this case the ring buffer can be split into two at this point. The first half contains
            // the tail of the ring buffer. And the second half contains head of the ring buffer.
            let (second, first) = memory.split_at(self.next * T::SIZE);
            (T::from_byte_slice(first), T::from_byte_slice(second))
        }
    }

    /// An iterator to iterate through all the items currently in ring buffer.
    pub fn iter(&self) -> ItemIterator<'_, T> {
        let (head, tail) = self.as_slices();
        ItemIterator {
            head,
            tail,
            next: Index::Head(0),
        }
    }
}

/// Index to an item in the ring buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Index {
    Head(usize),
    Tail(usize),
}

/// An iterator to iterate through items in a ring buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemIterator<'a, T> {
    // Reference to the items currently held in the ring buffer.
    head: &'a [T],
    tail: &'a [T],

    // Next index to read records from.
    next: Index,
}

impl<'a, T: Item> Iterator for ItemIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next {
            // Reached end of the iterator, nothing more to return.
            Index::Tail(index) if index == self.tail.len() => None,
            Index::Head(index) if index == self.head.len() && self.tail.is_empty() => None,

            // Next read from tail.
            // Safety: Check above to make sure read is not out of bounds.
            Index::Tail(index) => {
                self.next = Index::Tail(index + 1);
                unsafe { Some(self.tail.get_unchecked(index)) }
            }

            // Next read from the start of the tail.
            // Safety: Check above to make sure read is not out of bounds.
            Index::Head(index) if index == self.head.len() => {
                self.next = Index::Tail(1);
                unsafe { Some(self.tail.get_unchecked(0)) }
            }

            // Read read from head.
            // Safety: Check above to make sure read is not out of bounds.
            Index::Head(index) => {
                self.next = Index::Head(index + 1);
                unsafe { Some(self.head.get_unchecked(index)) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::{TypeGenerator, check};
    use ringbuffer::{AllocRingBuffer, RingBuffer};
    use rstest::rstest;
    use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

    #[derive(
        Debug,
        Clone,
        Copy,
        PartialEq,
        Eq,
        FromBytes,
        IntoBytes,
        Immutable,
        KnownLayout,
        TypeGenerator,
    )]
    struct Log(u64);

    impl Item for Log {
        const SIZE: usize = size_of::<Log>();

        fn to_bytes_slice(records: &[Self]) -> &[u8] {
            records.as_bytes()
        }

        fn from_byte_slice(bytes: &[u8]) -> &[Self] {
            <[Self]>::ref_from_bytes(bytes).expect("Should transmute back to items")
        }
    }

    /// A reference implementation of ring buffer from a popular crate.
    struct Oracle<T>(AllocRingBuffer<T>);

    impl<T: Item> Oracle<T> {
        fn with_capacity(capacity: usize) -> Self {
            Self(AllocRingBuffer::new(capacity))
        }

        fn append_from_slice(&mut self, items: &[T]) {
            self.0.extend(items.iter().map(|item| *item));
        }
    }

    #[rstest]
    #[case(32)]
    #[case(512)]
    #[case(1024)]
    fn state_machine(#[case] capacity: usize) {
        check!()
            .with_type::<Vec<Vec<Log>>>()
            .for_each(|operations| {
                // Ring buffers for equivalence testing.
                let mut hadron = Hadron::with_capacity(capacity);
                let mut oracle = Oracle::with_capacity(capacity);

                // Process the batch of items.
                for items in operations {
                    // Copy the batch of items into the ring buffer.
                    hadron.append_from_slice(items);
                    oracle.append_from_slice(items);

                    // Make sure items are the same between the ring buffers.
                    let hadron_items: Vec<_> = hadron.iter().collect();
                    let oracle_items: Vec<_> = oracle.0.iter().collect();
                    assert_eq!(hadron_items, oracle_items);
                }
            });
    }
}
