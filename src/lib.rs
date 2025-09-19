//! # Crosstream
//!
//! Crosstream provides different types of ring buffers along with primitives to build them yourself.
//!
//! ## SeqRing
//!
//! A [`SeqRing`] is a ring buffer that holds sequential records of type [`SeqRecord`]. It is like any
//! other ring buffer, except it is custom built for efficient query via record sequence numbers.
//!
//! ### Storage engine
//!
//! A [`SeqRing`] provides different types of storage engines based on your needs.
//!
//! * [`VecSeqRing`] - A SeqRing that uses [`Vec`] to store records.
//! * [`OnHeapSeqRing`] - A SeqRing that uses on heap memory via global allocator to store records.
//! * [`OffHeapSeqRing`] - A SeqRing that uses off heap memory via anonymous mmap to store records.
//!
//! ## Record
//!
//! A [`Record`] is a fixed size element that can be stored in a ring buffer. It has compile
//! time known size and alignment. This allows for certain types of optimization, including
//! zero-copy transmutation turning most ring buffer operations into pure memcpy.
//!
//! A [`SeqRecord`] is a special type of [`Record`] that has a sequence number attached to it.
//!
//! ### Features
//!
//! There is a blanket implementation for [`Record`] for supported types from popular crates. This can be
//! activated with one of the feature flags below. Note only one of the features can be enabled, not all.
//!
//! * `zerocopy` - For types that implement supported traits from [`zerocopy`](https://docs.rs/zerocopy/latest/zerocopy/)
//! * `bytemuck` - For types that implement supported traits from [`bytemuck`](https://docs.rs/bytemuck/latest/bytemuck/)
//!
//! ## QueryBuf
//!
//! You don't want to repeatedly heap allocate when repeatedly querying for records from a ring buffer.
//! [`QueryBuf`] is a reusable buffer that can be used to fast memcpy records from ring buffer. As of
//! now this is the only way to query for records from a ring buffer.

// Internally exposed modules.
pub(crate) mod buf;
pub(crate) mod experiment;
pub(crate) mod record;
pub(crate) mod ring;
pub(crate) mod storage;

// Externally exposed types.
pub use buf::QueryBuf;
pub use experiment::hadron::{Hadron, Item};
pub use record::{Record, SeqRecord};
pub use ring::{AppendError, OffHeapSeqRing, OnHeapSeqRing, SeqRing, VecSeqRing};
pub use storage::{
    MemStorage, OffHeap, OffHeapStorage, OnHeap, OnHeapStorage, Storage, VecStorage,
};
