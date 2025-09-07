//! # Ring

pub(crate) mod record;
pub(crate) mod segment;

// Externally exposed types.
pub use record::Record;
pub use segment::{MmapSegment, MmapStorage, Segment, Trimmer, VecSegment, VecStorage};
