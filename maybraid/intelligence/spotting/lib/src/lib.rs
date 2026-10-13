//! Physics-independent spotting policy, semantic subjects, and contact memory.
//!
//! Add [`SpotSubject`] to discoverable entities and [`SpottingUser`] to
//! observers. A backend such as `spotting-intelligence-avian` performs
//! broadphase discovery and visibility probes.

mod bounds;
mod candidate;
mod contact;
mod directive;
mod layers;
mod subject;
mod user;

use bevy::prelude::*;

pub use bounds::{SpotBounds, SpotFeature, SpotSample};
pub use candidate::SpotCandidate;
pub use contact::SpottedContact;
pub use directive::{SpotContactView, SpotDirective};
pub use layers::InterestLayers;
pub use subject::SpotSubject;
pub use user::{SpottingHint, SpottingHintSource, SpottingSettings, SpottingUser};

/// Shared schedule location for spotting backends.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpottingSystems {
	Observe,
}

/// How many due observers may start discovery on one Observe tick.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpottingObserveLimits {
	pub max_observers_per_tick: usize,
}

impl Default for SpottingObserveLimits {
	fn default() -> Self {
		Self { max_observers_per_tick: 8 }
	}
}
