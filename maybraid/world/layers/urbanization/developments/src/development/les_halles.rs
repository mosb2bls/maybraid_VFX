//! [`LesHalles`]: one mixed-use hall on a yawed terrace.

mod hall;

pub use hall::{courtyard_well_side, MixedUseLesHallesDevelopment, MixedUseLesHallesHost};

use buildings::Fit;
use procedural_common::SeededHash;

use crate::{DevelopmentFinish, PlacedBuilding, Terrace, TerraceDevelopment, TerraceEnvelope};

/// Les Halles keeps its original urban envelope inside the larger shared cell.
pub const LES_HALLES_MAX_FOOTPRINT: f32 = 72.0;
/// Minimum footprint on each plan axis (metres).
const MIN_FOOTPRINT: f32 = 36.0;
/// Confines height range (2–7 storeys at 3–5 m).
const MIN_CONFINES_HEIGHT: f32 = 10.0;
const MAX_CONFINES_HEIGHT: f32 = 35.0;

/// One mixed-use hall on a yawed terrace.
pub type LesHalles = PlacedBuilding<MixedUseLesHallesDevelopment>;

impl TerraceDevelopment for LesHalles {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: MIN_FOOTPRINT,
			max_footprint: LES_HALLES_MAX_FOOTPRINT.max(MIN_FOOTPRINT),
			min_height: MIN_CONFINES_HEIGHT,
			max_height: MAX_CONFINES_HEIGHT,
			rotates: true,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick(hash)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		let (placed, _) = Self::fit_to_confines(&terrace.confines(), terrace.noise()).ok()?;
		let finish = terrace.finish.clone();
		Some(placed.map(|halles| halles.with_finish(finish.wall, finish.roof)))
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::TAU;

	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use material_ref::MaterialId;

	use super::*;
	use crate::yawed_plan_aabb_extent;

	fn terrace(seed: u32) -> Terrace {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		Terrace::new::<LesHalles>(cell, 12.0, seed)
	}

	#[test]
	fn terrace_picks_urban_finish() {
		let finish = terrace(42).finish;
		assert!(matches!(
			&finish.wall.name,
			MaterialId::Name(n) if n == "stucco" || n == "wood"
		));
		assert!(matches!(
			&finish.roof.name,
			MaterialId::Name(n) if n == "iron" || n == "terracotta" || n == "hay"
		));
	}

	#[test]
	fn terrace_samples_continuous_yaw() {
		let eighth = TAU / 8.0;
		let mut off_grid = false;
		for seed in 0..48u32 {
			let terrace = terrace(seed);
			assert!(terrace.confines_yaw >= 0.0 && terrace.confines_yaw <= TAU + 1e-5);
			let phase = terrace.confines_yaw.rem_euclid(eighth);
			if phase > 0.05 && phase < eighth - 0.05 {
				off_grid = true;
			}
			let pad = LES_HALLES_MAX_FOOTPRINT;
			let occupied = yawed_plan_aabb_extent(
				terrace.confines_extent_xz.x,
				terrace.confines_extent_xz.y,
				terrace.confines_yaw,
			);
			assert!(occupied.x <= pad + 1e-3, "yawed AABB x {} exceeds pad {}", occupied.x, pad);
			assert!(occupied.y <= pad + 1e-3, "yawed AABB z {} exceeds pad {}", occupied.y, pad);
			assert!((terrace.confines().roll - terrace.confines_yaw).abs() < 1e-6);
		}
		assert!(off_grid, "expected at least one heading off the old π/4 lattice");
	}
}
