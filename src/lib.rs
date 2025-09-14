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

pub(crate) mod record;
pub(crate) mod storage;

// Externally exposed types.
pub use record::Record;
pub use storage::{
    MemStorage, OffHeap, OffHeapStorage, OnHeap, OnHeapStorage, Storage, VecStorage,
};
