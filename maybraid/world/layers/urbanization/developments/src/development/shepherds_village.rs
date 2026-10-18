//! [`ShepherdsVillage`]: huts and houses scattered on their own pads.

mod building;
pub(super) mod fit;

pub use building::{
	ShepherdsBuilding, ShepherdsFinish, ShepherdsHouse, ShepherdsHut, ShepherdsVillageBuilding,
	HOUSE_MAX_FOOTPRINT, HOUSE_MIN_FOOTPRINT, HOUSE_STOREY_HEIGHT, HUT_HEIGHT, HUT_MAX_FOOTPRINT,
	HUT_MIN_FOOTPRINT,
};

use bevy_math::bounding::Aabb3d;
use procedural_common::{Bounds2, NoiseParams, SeededHash};

use crate::plan::cell_salt;
use crate::scatter::bounds_intersect;
use crate::{Development, PadParams, PadPlan, SiteGround};
use fit::{fit_shepherds_building, shepherds_recipe};

/// A 300 m development-cell village.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsVillage {
	pub bounds: Aabb3d,
	pub buildings: Vec<ShepherdsVillageBuilding>,
}

impl ShepherdsVillage {
	pub fn new(bounds: Aabb3d, buildings: Vec<ShepherdsVillageBuilding>) -> Self {
		Self { bounds, buildings }
	}
}

impl Development for ShepherdsVillage {
	type Plan = Self;

	/// Deterministic 4×4 jittered huts and houses on `cell`, each on its own
	/// pad seated over `ground` and kept clear of water.
	fn plan(ground: &mut impl SiteGround, cell: Aabb3d, seed: u32) -> Option<(Self, Vec<PadPlan>)> {
		let root = SeededHash::new(seed.wrapping_add(cell_salt(cell)));
		let recipe = shepherds_recipe();
		let plan = recipe.plan(cell, root);

		let mut buildings = Vec::with_capacity(plan.target_count);
		let mut pads = Vec::with_capacity(plan.target_count);
		let mut occupied = Vec::<Bounds2>::with_capacity(plan.target_count);
		for candidate in plan.candidates {
			if buildings.len() >= plan.target_count {
				break;
			}
			let hash = SeededHash::new(
				root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0x9E37_79B9)),
			);
			let occupied_bounds = recipe.collision_bounds(&candidate);
			if occupied.iter().any(|b| bounds_intersect(*b, occupied_bounds)) {
				continue;
			}

			let coarse_pad = PadPlan::building_skirt(
				candidate.center,
				candidate.footprint * 0.5,
				candidate.yaw,
				0.0,
				PadParams::shepherds(),
			);
			if ground.hydro_overlaps(&coarse_pad) {
				continue;
			}
			let Some(height) = ground.height_upper_on_rect(
				candidate.center,
				PadParams::shepherds().influence_half(candidate.footprint * 0.5),
				candidate.yaw,
			) else {
				continue;
			};

			let noise = NoiseParams {
				seed: seed as i32 ^ (candidate.slot as i32 * 7919),
				..NoiseParams::default()
			};
			let Some(placed) = fit_shepherds_building(
				candidate.kind,
				candidate.center,
				candidate.yaw,
				candidate.footprint,
				height,
				hash,
				noise,
			) else {
				continue;
			};
			let pad = placed.pad_plan(PadParams::shepherds());
			if ground.hydro_overlaps(&pad) {
				continue;
			}
			buildings.push(placed);
			pads.push(pad);
			occupied.push(occupied_bounds);
		}

		if buildings.is_empty() {
			None
		} else {
			Some((Self::new(cell, buildings), pads))
		}
	}

	fn build(village: &Self) -> Option<Self> {
		Some(village.clone())
	}
}

#[cfg(test)]
mod tests {
	use bevy_math::{Vec2, Vec3};

	use super::fit::ShepherdsBuildingKind;
	use super::*;
	use crate::ground::tests::FlatGround;
	use crate::scatter::ScatterCandidate;

	#[test]
	fn flat_dry_ground_places_buildings_on_their_pads() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let mut ground = FlatGround { height: 9.0, wet: false };
		let (village, pads) = ShepherdsVillage::plan(&mut ground, cell, 42)
			.ok_or_else(|| anyhow::anyhow!("village on flat ground"))?;
		anyhow::ensure!(!village.buildings.is_empty());
		anyhow::ensure!(pads.len() == village.buildings.len());
		anyhow::ensure!(pads.iter().all(|pad| (pad.height - 9.0).abs() < 1e-3));
		let mut wet = FlatGround { height: 9.0, wet: true };
		anyhow::ensure!(
			ShepherdsVillage::plan(&mut wet, cell, 42).is_none(),
			"water everywhere rejects every hut"
		);
		Ok(())
	}

	#[test]
	fn jittered_centers_stay_inside_the_cell_inset() {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
		let plan = shepherds_recipe().plan(cell, SeededHash::new(0));
		for candidate in plan.candidates {
			let center = candidate.center;
			assert!((32.0..=168.0).contains(&center.x));
			assert!((32.0..=168.0).contains(&center.y));
		}
	}

	#[test]
	fn continuous_yaw_changes_the_collision_envelope() {
		let recipe = shepherds_recipe();
		let mut candidate = ScatterCandidate {
			slot: 0,
			center: Vec2::splat(100.0),
			yaw: 0.0,
			footprint: Vec2::new(24.0, 12.0),
			kind: ShepherdsBuildingKind::House,
		};
		let aligned = recipe.collision_bounds(&candidate);
		candidate.yaw = std::f32::consts::FRAC_PI_4;
		let yawed = recipe.collision_bounds(&candidate);
		assert_ne!(aligned.max - aligned.min, yawed.max - yawed.min);
	}
}
