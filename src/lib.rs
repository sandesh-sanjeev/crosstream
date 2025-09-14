//! # Crosstream
//!
//! Crosstream provides different types of ring buffers along with primitives to build them yourself.
//!
//! ## Record
//!
//! A [`Record`] is a fixed size element that can be stored in a ring buffer. It has compile
//! time known size and alignment, which allows for certain types of optimization.
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
pub(crate) mod record;
pub(crate) mod storage;

// Externally exposed types.
pub use buf::QueryBuf;
pub use record::{Record, SeqRecord};
pub use storage::{
    MemStorage, OffHeap, OffHeapStorage, OnHeap, OnHeapStorage, Storage, VecStorage,
};
