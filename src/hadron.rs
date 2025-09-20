//! Definition of a ring buffer.

use crate::{Heap, Memory};
use std::{cmp::min, marker::PhantomData, mem::needs_drop};

/// Hadron is a fixed size ring buffer.
///
/// It is designed for high performance use cases and makes trade-offs to achieve it.
/// Bulk append and copy is guaranteed to be exactly 2 memcpy operations. Additionally
/// provides reference to all the items held in constant time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hadron<T, Alloc = Heap<T>> {
    // Index where the next append will occur.
    // This will wrap around to 0 when next == cap.
    next: usize,

    // Number of records currently held in the ring buffer.
    length: usize,

    // A pre-allocated memory for ring buffer records.
    memory: Alloc,

    // Type of record held in the ring buffer.
    phantom: PhantomData<T>,
}

impl<T> Hadron<T> {
    /// Create a new instance of this ring buffer.
    ///
    /// All required memory is allocated during initialization. It is
    /// guaranteed that no allocations happen after initialization.
    ///
    /// # Panic
    ///
    /// * Ring buffer must have at least one item.
    /// * Number of items in bytes should be <= isize::MAX.
    /// * Only trivially droppable types supported.
    /// * Ring buffer capacity has to be a power of 2.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items this ring buffer can hold.
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be > 0");
        assert!(!needs_drop::<T>(), "Item must be trivially droppable");
        assert!(capacity.is_power_of_two(), "Capacity should be power of 2");

        Self {
            next: 0,
            length: 0,
            phantom: PhantomData,
            memory: Heap::alloc(capacity),
        }
    }
}

impl<T: Copy, Alloc: Memory<T>> Hadron<T, Alloc> {
    /// Append a slice of items into this ring buffer.
    ///
    /// If newly appended records exceeds the capacity of this ring buffer,
    /// space is reclaimed by evicting old records from the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `items` - Items to append into this ring buffer.
    pub fn append_from_slice(&mut self, mut items: &[T]) {
        // Get reference to the memory that holds ring buffer items.
        let memory = self.memory.as_mut();

        // If number of items is greater than the capacity of this ring buffer, some of the items
        // will be overwritten. We can optimize this by skipping those items. This also allows us
        // to make this append at exactly 2 memcpy operations.
        if items.len() > memory.len() {
            let split = items.len() - memory.len();
            items = items.split_at(split).1;
        }

        // When we reach the end of the ring buffer, we wrap around and overwrite oldest items.
        // Which means we need exactly 2 memcpy operations. One from current index till end of
        // the buffer. Another one to start write from index of 0.
        let remaining = memory.len() - self.next;
        let (first, second) = match items.split_at_checked(remaining) {
            Some(split) => split,
            None => (items, Default::default()),
        };

        // Split the backing memory into discrete writeable chunks.
        let (tail, head) = memory.split_at_mut(self.next);

        // Write the relevant portions of items into those chunks.
        head[..first.len()].copy_from_slice(first);
        tail[..second.len()].copy_from_slice(second);

        // Update state.
        self.next = (self.next + items.len()) & (memory.len() - 1);
        self.length = min(self.length + items.len(), memory.len());
    }
}

impl<T, Alloc: Memory<T>> Hadron<T, Alloc> {
    /// Get a reference to items currently stored in the ring buffer.
    ///
    /// Since the ring buffer can wrap around, items in the ring buffer are stored
    /// in two non-overlapping discrete chunks of items. When the ring buffer is not
    /// full, tail is always empty.
    pub fn as_slices(&self) -> (&[T], &[T]) {
        // Get immutable reference to the underlying items.
        let memory = self.memory.as_ref();

        // If the ring buffer has not wrapped around, the starting index is always 0.
        let (second, first) = if self.length < memory.len() {
            // There is only head in this case, no tail.
            // Because ring buffer has not wrapped around 0 yet, it's just a single slice.
            (Default::default(), &memory[..self.length])
        } else {
            // If the ring buffer has indeed wrapped around, then starting index is the same as next.
            // In this case the ring buffer can be split into two at this point. The first half contains
            // the tail of the ring buffer. And the second half contains head of the ring buffer.
            memory.split_at(self.next)
        };

        // Return head and tail reversed,
        // cause that's the natural way to think about it.
        (first, second)
    }

    /// An iterator to iterate through all the items currently in ring buffer.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let (head, tail) = self.as_slices();
        head.iter().chain(tail.iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bolero::{TypeGenerator, check};
    use ringbuffer::{AllocRingBuffer, RingBuffer};
    use rstest::rstest;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, TypeGenerator)]
    struct Log(u64);

    /// A reference implementation of ring buffer from a popular crate.
    struct Oracle<T>(AllocRingBuffer<T>);

    impl<T: Copy> Oracle<T> {
        fn with_capacity(capacity: usize) -> Self {
            Self(AllocRingBuffer::new(capacity))
        }

        fn append_from_slice(&mut self, items: &[T]) {
            self.0.extend(items.iter().map(|item| *item));
        }

        fn iter(&self) -> impl Iterator<Item = &T> {
            self.0.iter()
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
                    let oracle_items: Vec<_> = oracle.iter().collect();
                    assert_eq!(hadron_items, oracle_items);
                }
            });
    }
}
