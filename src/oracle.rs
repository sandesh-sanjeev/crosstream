//! Reference implementations of ring buffer using standard library primitives.

use std::collections::VecDeque;

/// A fixed size ring buffer backed by [`VecDeque`].
pub struct Oracle<T> {
    capacity: usize,
    deque: VecDeque<T>,
}

impl<T> Oracle<T> {
    /// Create a new instance of this ring buffer.
    ///
    /// All required memory is allocated during initialization. It is
    /// guaranteed that no allocations happen after initialization.
    ///
    /// # Panic
    ///
    /// * Ring buffer must have at least one item.
    /// * Number of items in bytes should be <= isize::MAX.
    /// * Does not support ZSTs.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of items this ring buffer can hold.
    #[track_caller]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(size_of::<T>() > 0, "Should not be ZST");
        assert!(capacity > 0, "Capacity must be > 0");

        Self {
            capacity,
            deque: VecDeque::with_capacity(capacity),
        }
    }

    /// An iterator to iterate through all the items currently in ring buffer.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.deque.iter()
    }
}

impl<T: Copy> Oracle<T> {
    /// Append a slice of items into this ring buffer.
    ///
    /// If newly appended records exceeds the capacity of this ring buffer,
    /// space is reclaimed by evicting old records from the ring buffer.
    ///
    /// # Arguments
    ///
    /// * `items` - Items to append into this ring buffer.
    #[inline]
    pub fn copy_from_slice(&mut self, mut items: &[T]) {
        // Skip items that will never be visible in this ring buffer.
        if items.len() > self.capacity {
            let split = items.len() - self.capacity;
            items = items.split_at(split).1;
        }

        // Make space in the ring buffer for this batch of items.
        let remaining = self.capacity - self.deque.len();
        if items.len() > remaining {
            self.deque.drain(..(items.len() - remaining));
        }

        // Append all items items into the ring buffer in one shot.
        self.deque.extend(items.iter().copied());
    }
}
