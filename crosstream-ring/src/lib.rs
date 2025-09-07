//! # Ring

pub(crate) mod record;
pub(crate) mod ring;
pub(crate) mod segment;

// Externally exposed types.
pub use record::Record;
pub use segment::Segment;
