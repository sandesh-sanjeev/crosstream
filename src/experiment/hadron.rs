use crate::experiment::memory::{Heap, Memory};
use std::marker::PhantomData;

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

        // Write the first batch.
        let first_bytes = T::to_bytes_slice(first);
        let first_start = self.next * T::SIZE;
        let first_end = first_start + first_bytes.len();
        self.memory.as_mut()[first_start..first_end].copy_from_slice(first_bytes);

        // Write the second batch.
        let second_bytes = T::to_bytes_slice(second);
        let second_end = second_bytes.len();
        self.memory.as_mut()[..second_end].copy_from_slice(second_bytes);

        // Update state.
        self.length = std::cmp::min(self.length + items.len(), self.capacity);
        self.next = (self.next + items.len()) % self.capacity;
    }

    /// Query items starting from the beginning of the ring buffer.
    ///
    /// Items are returned in the same order that then were appended into the ring buffer.
    /// Note that the buffer to write items into will be cleared to make space for records,
    /// even if there were no matching records. However no heap allocations occur, we only
    /// write to buffers capacity.
    ///
    /// # Arguments
    ///
    /// * `buf` - Buffer to copy items into.
    pub fn query_from_trim(&self, buf: &mut Vec<T>) {
        buf.clear(); // Make space for new records.

        // Get a reference to all the items currently held in the ring buffer.
        let (head, tail) = self.item_slices();

        // Copy records from head of the ring buffer.
        let len = std::cmp::min(head.len(), buf.capacity());
        buf.extend(&head[..len]);

        // Copy records from tail of the ring buffer.
        let remaining = buf.capacity() - len;
        let len = std::cmp::min(tail.len(), remaining);
        buf.extend(&tail[..len]);
    }

    /// Fetch items records currently stored in the ring buffer.
    fn item_slices(&self) -> (&[T], &[T]) {
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
}

#[cfg(test)]
#[cfg(feature = "zerocopy")]
mod tests {
    use super::*;
    use bolero::{TypeGenerator, check};
    use std::collections::VecDeque;
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
            <[Self]>::ref_from_bytes(bytes)
                .expect("Should be able to transmute byte slice to a slice of items")
        }
    }

    struct RefHadron<T> {
        capacity: usize,
        deque: VecDeque<T>,
    }

    impl<T: Item> RefHadron<T> {
        fn new(capacity: usize) -> Self {
            Self {
                capacity,
                deque: VecDeque::with_capacity(capacity),
            }
        }

        fn append_from_slice(&mut self, items: &[T]) {
            for item in items {
                // Clear space for the new item.
                if self.deque.len() == self.capacity {
                    self.deque.pop_front();
                }

                // Append the new item into the ring buffer.
                self.deque.push_back(*item);
            }
        }

        fn query_from_trim(&mut self, buf: &mut Vec<T>) {
            // Make space for new items.
            buf.clear();

            for item in self.deque.iter() {
                // Quit if we have collected enough items.
                if buf.len() == buf.capacity() {
                    break;
                }

                // Collect the item.
                buf.push(*item);
            }
        }
    }

    #[test]
    fn state_machine() {
        check!()
            .with_type::<Vec<Vec<Log>>>()
            .for_each(|operations| {
                let mut hadron = Hadron::with_capacity(32);
                let mut ref_hadron = RefHadron::new(32);

                for items in operations {
                    hadron.append_from_slice(items);
                    ref_hadron.append_from_slice(items);
                }

                let mut hadron_buf = Vec::with_capacity(32);
                hadron.query_from_trim(&mut hadron_buf);

                let mut ref_hadron_buf = Vec::with_capacity(32);
                ref_hadron.query_from_trim(&mut ref_hadron_buf);

                assert_eq!(hadron_buf, ref_hadron_buf);
            });
    }

    #[test]
    fn experimental_ring() {
        // Initialize the ring buffer.
        let mut hadron = Hadron::with_capacity(1024);

        // Test records to append into the ring buffer.
        let records: Vec<Log> = (0..=2048).map(Log).collect();

        // Write records in chunks.
        for chunk in records.chunks(64) {
            hadron.append_from_slice(chunk);
        }
    }

    #[test]
    fn query_from_trim() {
        // Initialize the ring buffer.
        let mut hadron = Hadron::with_capacity(1024);

        // Query for records from the ring buffer.
        let mut buf = Vec::with_capacity(1024);
        hadron.query_from_trim(&mut buf);

        // Populate ring buffer with logs.
        let logs: Vec<_> = (1..=1024 as u64).map(Log).collect();
        hadron.append_from_slice(&logs);

        // Query for records with different batch sizes.
        for batch_size in (256..=1024).step_by(256) {
            // Buffer to query for records from the ring.
            let mut buf = Vec::with_capacity(batch_size);

            // Query from the very beginning.
            hadron.query_from_trim(&mut buf);

            // Make sure expected records were returned.
            assert_eq!(&logs[..batch_size], &buf);
        }
    }
}
