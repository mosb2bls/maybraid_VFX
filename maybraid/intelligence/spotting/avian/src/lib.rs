//! Avian broadphase discovery and fixed-geometry line-of-sight for
//! `spotting-intelligence`.

mod candidate;
mod discover;
mod los;
mod observe;
mod probe;

use bevy::prelude::*;
use intelligence_lod::IntelligencePriority;
use spotting_intelligence::{SpottingObserveLimits, SpottingSystems};

pub use los::{clear_segment, RAY_ORIGIN_SKIP};
pub use observe::observe_spotting;

/// Installs Avian-backed observation in [`SpottingSystems::Observe`].
pub struct SpottingAvianPlugin;

impl Plugin for SpottingAvianPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<IntelligencePriority>()
			.init_resource::<SpottingObserveLimits>()
			.add_systems(Update, observe_spotting.in_set(SpottingSystems::Observe));
	}
}
