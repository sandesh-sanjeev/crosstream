//! # crosstream-ring
//!
//! `crosstream-ring` provides ring buffers along with primitives to build them yourself.
//!
//! ## Record
//!
//! A [`Record`] is a fixed size element that can be stored in a ring buffer. It has compile
//! time known size and alignment, which allows for certain types of optimization.
//!
//! As of now only types that implement this trait can be used with ring buffers. There is a
//! blanket implementation for support typed in [`bytemuck`](https://docs.rs/bytemuck/latest/bytemuck/),
//! will continue to add default support for more types.
//!
//! ## Segment
//!
//! A [`Segment`] is a contagious collection of records. A Segment has a fixed capacity that is determined
//! during initialization. A Segment cannot grow beyond initially allocated capacity, however space can be
//! reclaimed via trimming. Trimming adds latency and hurts throughput. Unless a Segment is small and/or
//! infrequently updated, prefer to use a Segment for append only storage.
//!
//! ### Trimmer
//!
//! A [`Trimmer`] a strategy used to determine trimming, if any, when a Segment does not have enough
//! space to hold records being appended. Note that one or more records might be rejected even after
//! trimming, if records trimmed < records being appended.
//!
//! * [`Trimmer::Nothing`] - Do nothing.
//! * [`Trimmer::Trim`] - Remove N records from the beginning of Segment.
//!
//! ### Storage
//!
//! A [`Segment`] can be initialized with different types of storage engines. Which one to use depends on
//! your specific use case. Every storage engine has roughly the same performance characteristics. Your choice
//! will depend on other factors, for example [`MmapStorage`] engine allows allocating memory with huge pages.
//!
//! * [`VecStorage`] - Storage engine backed by [`Vec`] with global allocator.
//! * [`MmapStorage`] - Storage engine backed by anonymous mmap for memory.
//!
//! ### Example
//!
//!```
//! use crosstream_ring::{MmapSegment, Trimmer};
//!
//! // Trimmer to use with a segment.
//! let trimmer = Trimmer::Nothing;
//!
//! // Create a new Segment.
//! let mut segment = MmapSegment::with_capacity(3, trimmer);
//!
//! // Append individual records.
//! assert_eq!(segment.push(1), None);
//! assert_eq!(segment.push(2), None);
//! assert_eq!(segment.push(3), None);
//! assert_eq!(segment.push(4), Some(4));
//!
//! // Read those records back.
//! assert_eq!(segment.records(), &[1, 2, 3]);
//!
//! // Trim the segment.
//! segment.trim(3);
//!
//! // Bulk append records.
//! assert_eq!(segment.extend_from_slice(&[4, 5, 6]), &[]);
//! assert_eq!(segment.extend_from_slice(&[7]), &[7]);
//!
//! // Read those records back.
//! assert_eq!(segment.records(), &[4, 5, 6]);
//!```

pub(crate) mod record;
pub(crate) mod segment;
pub(crate) mod storage;

// Externally exposed types.
pub use record::Record;
pub use segment::{MmapSegment, Segment, Trimmer, VecSegment};
pub use storage::{MmapStorage, Storage, VecStorage};
