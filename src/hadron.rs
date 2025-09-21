//! Definition of a ring buffer.

use crate::heap::Array;
use std::{cmp::min, fmt::Debug};

/// Hadron is a fixed size ring buffer.
///
/// It is designed for high performance use cases and makes trade-offs to achieve it.
/// Bulk append and copy is guaranteed to be exactly 2 memcpy operations. Additionally
/// provides reference to all the items held in constant time.
pub struct Hadron<T> {
    // Index where the next append will occur.
    // This will wrap around to 0 when next == cap.
    next: usize,

    // Number of records currently held in the ring buffer.
    length: usize,

    // A pre-allocated memory for ring buffer records.
    memory: Array<T>,
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
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items this ring buffer can hold.
    #[track_caller]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "Capacity must be > 0");

        Self {
            next: 0,
            length: 0,
            memory: Array::alloc(capacity),
        }
    }

    /// Get a reference to items currently stored in the ring buffer.
    ///
    /// Since the ring buffer can wrap around, items in the ring buffer are stored
    /// in two non-overlapping discrete chunks of items. When the ring buffer is not
    /// full, tail is always empty.
    #[inline]
    pub fn as_slices(&self) -> (&[T], &[T]) {
        // If the ring buffer has not wrapped around, the starting index is always 0.
        let capacity = self.memory.len();
        if self.length < capacity {
            // Head of the ring buffer.
            let head = self.memory.as_slice(0, self.length);

            (head, Default::default())
        } else {
            // Head of the ring buffer.
            let head = self.memory.as_slice(self.next, capacity - self.next);

            // Tail of the ring buffer.
            let tail = self.memory.as_slice(0, self.next);

            // Return both halves of the ring buffer.
            (head, tail)
        }
    }

    /// An iterator to iterate through all the items currently in ring buffer.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        let (head, tail) = self.as_slices();
        head.iter().chain(tail.iter())
    }
}

impl<T: Copy + Debug> Hadron<T> {
    /// Append a slice of items into this ring buffer.
    ///
    /// If newly appended records exceeds the capacity of this ring buffer,
    /// space is reclaimed by evicting old records from the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `items` - Items to append into this ring buffer.
    #[inline]
    pub fn copy_from_slice(&mut self, items: &[T]) {
        // Maximum bytes memory can accommodate.
        let capacity = self.memory.len();

        // Index of items from where writes can begin,
        let src = items.as_ptr();
        let src_start = items.len().saturating_sub(capacity);
        let src_count = items.len() - src_start;

        // Items that can be written till end of the ring buffer.
        let remaining = capacity - self.next;

        // If remaining is >= than number of items to write,
        // all of it can be written in one shot
        if remaining > src_count {
            // Everything can be copied in one shot.
            self.memory.memcpy(self.next, src, src_start, src_count);

            // Cannot wrap around since remaining > src_count.
            self.next += src_count;

            // To handle the case where ring buffer hasn't filled up yet.
            self.length = min(self.length + src_count, capacity);
        } else {
            // First write out items till the end of the ring buffer.
            self.memory.memcpy(self.next, src, src_start, remaining);

            // Then write out the rest.
            let tail_count = src_count - remaining;
            let tail_start = src_start + remaining;
            self.memory.memcpy(0, src, tail_start, tail_count);

            // For next iteration.
            self.next = tail_count;
            self.length = capacity;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Oracle;
    use bolero::{TypeGenerator, check, generator};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, TypeGenerator)]
    struct Log(u64);

    #[test]
    fn state_machine() {
        check!()
            .with_generator((
                generator::produce::<usize>().with().bounds(1..=1024),
                generator::produce::<Vec<Vec<Log>>>(),
            ))
            .for_each(|(capacity, operations)| {
                // Ring buffers for equivalence testing.
                let mut hadron = Hadron::with_capacity(*capacity);
                let mut oracle = Oracle::with_capacity(*capacity);

                // Process the batch of items.
                for items in operations {
                    // Copy the batch of items into the ring buffer.
                    hadron.copy_from_slice(items);
                    oracle.copy_from_slice(items);

                    // Make sure items are the same between the ring buffers.
                    let hadron_items: Vec<_> = hadron.iter().collect();
                    let oracle_items: Vec<_> = oracle.iter().collect();
                    assert_eq!(hadron_items, oracle_items);
                }
            });
    }
}
