//! Stick domain: geometry + placement → [`StickNode`].

pub mod collection;
pub mod geometry;
pub mod node;
pub mod probe;

pub use collection::{
	StickCollection, StickMember, STICK_COLLECTION_HIGH_METERS, STICK_COLLECTION_LOW_METERS,
	STICK_COLLECTION_MEDIUM_METERS,
};
pub use geometry::StickGeometry;
pub use node::StickNode;
pub use probe::{
	update_stick_host_levels, StickLodProbe, STICK_HIGH_FACTOR, STICK_LOW_FACTOR,
	STICK_MEDIUM_FACTOR,
};

pub use crate::procedural::STICK_KIT_HALF;
