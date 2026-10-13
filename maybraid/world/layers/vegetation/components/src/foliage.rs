//! Foliage domain: geometry + placement → [`FoliageNode`].
//!
//! A kit collection is members + probe. [`CollectionPresent`] on the node chooses
//! one merged mesh or instanced kits under that same host.

pub mod collection;
pub mod geometry;
pub mod node;
pub mod present;
pub mod probe;

pub use collection::{
	CheapBallCollection, FrondCollection, FrondKit, FrondMember, FrondRun,
	CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS, COLLECTION_HIGH_METERS, COLLECTION_LOW_METERS,
	COLLECTION_MEDIUM_METERS, FROND_COLLECTION_HIGH_FACTOR, FROND_COLLECTION_HIGH_METERS,
	FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_LOW_METERS, FROND_COLLECTION_MEDIUM_FACTOR,
	FROND_COLLECTION_MEDIUM_METERS,
};
pub use geometry::FoliageGeometry;
pub use node::FoliageNode;
pub use present::CollectionPresent;
pub use probe::{
	update_foliage_host_levels, FoliageLodProbe, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR,
	FOLIAGE_MEDIUM_FACTOR,
};
