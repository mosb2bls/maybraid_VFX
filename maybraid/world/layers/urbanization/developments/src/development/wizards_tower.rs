//! [`WizardsTower`]: one solitary wizard's tower on a yawed terrace.

mod tower;

pub use tower::SolitaryWizardsTower;

use buildings::Fit;
use procedural_common::SeededHash;

use super::ring_fort::RING_FORT_MAX_FOOTPRINT;
use crate::{
	DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, Terrace, TerraceDevelopment,
	TerraceEnvelope,
};

/// One solitary wizard's tower on a yawed terrace.
pub type WizardsTower = PlacedBuilding<SolitaryWizardsTower>;

impl TerraceDevelopment for WizardsTower {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 24.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(42.0),
			min_height: 48.0,
			max_height: 104.0,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::WizardsTower, false)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		let (placed, _) = Self::fit_to_confines(&terrace.confines(), terrace.noise()).ok()?;
		let finish = terrace.finish.clone();
		Some(placed.map(|tower| tower.with_finish(finish.wall, finish.roof)))
	}
}
