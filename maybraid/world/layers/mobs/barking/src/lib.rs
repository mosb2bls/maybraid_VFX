//! Environment-driven groups that place generated semantic mob scenes.
//!
//! [`Barking<G>`] is the mob model. It samples lower layers through traits it
//! owns, implemented here for the concrete stack it sits on.

mod config;
mod generation;
pub mod hcsg;
mod index;
mod plugin;
mod present;
mod sample;

pub use config::BarkingConfig;
pub use generation::{
	GroupKind, MobEnvironmentSample, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample,
	PlacedMob, DEFAULT_GROUP_EXTENT,
};
pub use hcsg::{
	BarkingPresentationPlugin, MobGround, MobNeighborhood, MobScenePresentationPlugin,
	PlacedMobCell, MOB_PRESENT_RADIUS,
};
pub use index::{MobCell, MobCellExtent};
pub use plugin::{MobGroupsPlugin, PendingMobGroups};
pub use present::{MobCellRoot, MobGroupRoot};

#[cfg(test)]
mod tests;
