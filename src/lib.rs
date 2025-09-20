//! # Crosstream
//!
//! Crosstream provides different types of ring buffers along with primitives to build them yourself.

// Internally exposed modules.
pub(crate) mod hadron;
pub(crate) mod memory;

// Externally exposed types.
pub use hadron::{Hadron, Oracle};
