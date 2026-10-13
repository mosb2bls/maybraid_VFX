//! [`SingleHighrise`]: one tower on a yawed terrace.

use bevy_math::bounding::Aabb2d;
use buildings::Fit;
use procedural_common::SeededHash;

use super::ring_fort::RING_FORT_MAX_FOOTPRINT;
use crate::{
	BuildingFootprint, DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, Terrace,
	TerraceDevelopment, TerraceEnvelope,
};

/// One [`buildings::SingleHighrise`] tower on a yawed terrace.
pub type SingleHighrise = PlacedBuilding<buildings::SingleHighrise>;

impl TerraceDevelopment for SingleHighrise {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 42.5,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(65.0),
			min_height: 48.0,
			max_height: 104.0,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Highrise, false)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		let (placed, _) = Self::fit_to_confines(&terrace.confines(), terrace.noise()).ok()?;
		Some(placed.map(|tower| tower.with_wall_material(terrace.finish.wall.clone())))
	}
}

impl BuildingFootprint for buildings::SingleHighrise {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![self.tower.floor_plan.footprint_bounds()]
	}
}

#[cfg(test)]
mod tests {
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use buildings::Confines;
	use procedural_common::NoiseParams;

	use super::*;

	#[test]
	fn tower_fills_the_requested_vertical_envelope() -> anyhow::Result<()> {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-20.0, 0.0, -20.0),
			Vec3::new(20.0, 80.0, 20.0),
		));
		let (highrise, _) =
			buildings::SingleHighrise::fit_to_confines(&confines, NoiseParams::default())?;
		assert!(highrise.storey_count() >= 20);
		Ok(())
	}
}
