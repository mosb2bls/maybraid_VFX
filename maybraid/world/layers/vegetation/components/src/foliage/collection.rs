//! Kit collections: many placed kits, one foliage LOD parent.
//!
//! - [`frond`]: connected blade runs ([`FrondCollection`])
//! - [`cheap_ball`]: cheap-ball placements ([`CheapBallCollection`])
//!
//! Presentation (one merged mesh vs instanced kits) lives on the node
//! ([`crate::CollectionPresent`]), not on these types.

pub mod cheap_ball;
pub mod frond;

pub use cheap_ball::{
	CheapBallCollection, CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS,
};
pub use frond::{
	FrondCollection, FrondKit, FrondMember, FrondRun, FROND_COLLECTION_HIGH_FACTOR,
	FROND_COLLECTION_HIGH_METERS, FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_LOW_METERS,
	FROND_COLLECTION_MEDIUM_FACTOR, FROND_COLLECTION_MEDIUM_METERS,
};

/// Absolute-meter warm-root cull (viewer↔collection center). Shared by frond,
/// cheap-ball, and stick collections.
pub const COLLECTION_HIGH_METERS: f32 = 500.0;
/// See [`COLLECTION_HIGH_METERS`].
pub const COLLECTION_MEDIUM_METERS: f32 = 750.0;
/// See [`COLLECTION_HIGH_METERS`].
pub const COLLECTION_LOW_METERS: f32 = 1000.0;
