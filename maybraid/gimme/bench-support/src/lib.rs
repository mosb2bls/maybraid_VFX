//! Small development-only helpers for Gimme / LOD fill microbenches.
//!
//! This is not a harness framework. Scenario setup, destination lifetimes, and
//! correctness checks stay in the calling bench so hidden state cannot inflate
//! results.

mod history;
mod microbench;
mod spatial;

pub use history::{ExtendIntersectingFill, PairWalkFill};
pub use microbench::{
	AbReport, DurationStats, Microbench, PassLifetime, DEFAULT_REPEATS, DEFAULT_WARMUP,
};
pub use spatial::HostGrid;

#[cfg(feature = "bevy")]
mod bevy_sys;

#[cfg(feature = "bevy")]
pub use bevy_sys::{MessageBuffers, PersistentSystem};
