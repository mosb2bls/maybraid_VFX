//! LOD spatial identifiers and storage versions.
//!
//! Scene / refresh runtime lives in [`crate::scene`]; generation and
//! presentation run on [`crate::hcsg`].

mod id;
mod version;

#[cfg(test)]
pub mod tests;

pub use id::{Bytes, Cell, Id, OriginCell, OriginalId};
pub use version::Version;
