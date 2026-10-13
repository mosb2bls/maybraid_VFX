//! [`SuburbanHomes`]: a neighborhood of homes on one terrace.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use buildings::Confines;
use procedural_common::{Bounds2, NoiseParams, SeededHash};

use super::ring_fort::RING_FORT_MAX_FOOTPRINT;
use super::root_hash;
use super::shepherds_village::fit::{fit_suburban_building, ShepherdsBuildingKind};
use super::shepherds_village::ShepherdsVillageBuilding;
use crate::finish::SuburbanPaletteBias;
use crate::scatter::{bounds_intersect, ScatterChoice, ScatterRecipe};
use crate::{
	DevelopmentFinish, DevelopmentFinishRole, Terrace, TerraceDevelopment, TerraceEnvelope,
};

/// Detached homes with broad spacing and street-like rows.
#[derive(Debug, Clone, PartialEq)]
pub struct SuburbanHomes {
	pub bounds: Aabb3d,
	pub homes: Vec<ShepherdsVillageBuilding>,
	pub secondary_buildings: Vec<ShepherdsVillageBuilding>,
}

impl TerraceDevelopment for SuburbanHomes {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 190.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(230.0),
			min_height: 12.0,
			max_height: 16.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::SuburbanHome, false)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		Self::fit(terrace.cell, &terrace.confines(), terrace.noise())
	}
}

impl SuburbanHomes {
	pub fn buildings(&self) -> impl Iterator<Item = &ShepherdsVillageBuilding> {
		self.homes.iter().chain(&self.secondary_buildings)
	}

	/// Homes, then outbuildings between them, inside `confines` on `cell`.
	pub fn fit(cell: Aabb3d, confines: &Confines, noise: NoiseParams) -> Option<Self> {
		let bounds = Aabb2d {
			min: Vec2::new(confines.bounds.min.x, confines.bounds.min.z),
			max: Vec2::new(confines.bounds.max.x, confines.bounds.max.z),
		};
		let root = root_hash(cell, noise);
		let bias = SuburbanPaletteBias::select(root);
		let homes_recipe = ScatterRecipe {
			grid_side: 4,
			min_count: 8,
			max_count: 10,
			cell_inset: 27.0,
			jitter: 7.0,
			clearance: 8.0,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::House,
				weight: 1.0,
				min_footprint: 14.0,
				max_footprint: 23.0,
			}],
		};
		let (homes, occupied) = scatter_suburban(
			bounds,
			confines.bounds.min.y,
			noise,
			&homes_recipe,
			root,
			bias,
			Vec::new(),
		);
		if homes.is_empty() {
			return None;
		}

		let secondary_recipe = ScatterRecipe {
			grid_side: 5,
			min_count: 2,
			max_count: 3,
			cell_inset: 15.0,
			jitter: 9.0,
			clearance: 5.0,
			choices: vec![ScatterChoice {
				kind: ShepherdsBuildingKind::Hut,
				weight: 1.0,
				min_footprint: 5.0,
				max_footprint: 8.0,
			}],
		};
		let secondary_root = SeededHash::new(root.seed.wrapping_add(0x6A09_E667));
		let (secondary_buildings, _) = scatter_suburban(
			bounds,
			confines.bounds.min.y,
			noise,
			&secondary_recipe,
			secondary_root,
			bias,
			occupied,
		);
		Some(Self { bounds: confines.bounds, homes, secondary_buildings })
	}
}

fn scatter_suburban(
	bounds: Aabb2d,
	height: f32,
	noise: NoiseParams,
	recipe: &ScatterRecipe<ShepherdsBuildingKind>,
	root: SeededHash,
	bias: SuburbanPaletteBias,
	mut occupied: Vec<Bounds2>,
) -> (Vec<ShepherdsVillageBuilding>, Vec<Bounds2>) {
	let initial_occupied = occupied.len();
	let plan = recipe.plan_in_bounds(bounds, root);
	let mut buildings = Vec::new();
	for candidate in plan.candidates {
		if buildings.len() >= plan.target_count {
			break;
		}
		let collision = recipe.collision_bounds(&candidate);
		if !bounds_contains(bounds, collision)
			|| occupied.iter().copied().any(|other| bounds_intersect(other, collision))
		{
			continue;
		}
		let hash = SeededHash::new(
			root.seed.wrapping_add((candidate.slot as u32 + 1).wrapping_mul(0xA24B_AED5)),
		);
		let mut local_noise = noise;
		local_noise.seed = noise.seed.wrapping_add(candidate.slot as i32 * 97);
		if let Some(building) = fit_suburban_building(
			candidate.kind,
			candidate.center,
			candidate.yaw,
			candidate.footprint,
			height,
			hash,
			local_noise,
			bias,
		) {
			occupied.push(collision);
			buildings.push(building);
		}
	}
	debug_assert_eq!(occupied.len(), initial_occupied + buildings.len());
	(buildings, occupied)
}

fn bounds_contains(bounds: Aabb2d, inner: Bounds2) -> bool {
	inner.min.x >= bounds.min.x
		&& inner.max.x <= bounds.max.x
		&& inner.min.y >= bounds.min.y
		&& inner.max.y <= bounds.max.y
}

#[cfg(test)]
mod tests {
	use bevy_math::Vec3;

	use super::*;
	use crate::plan::yawed_plan_aabb_extent;
	use crate::{ShepherdsBuilding, ShepherdsFinish};

	fn shepherds_finish(building: &ShepherdsVillageBuilding) -> anyhow::Result<&ShepherdsFinish> {
		match &building.building {
			ShepherdsBuilding::House(house) => house.finish.as_ref(),
			ShepherdsBuilding::Hut(hut) => hut.finish.as_ref(),
		}
		.ok_or_else(|| anyhow::anyhow!("suburban building had no finish"))
	}

	#[test]
	fn suburban_homes_are_coherent_diverse_and_include_outbuildings() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let center = Vec2::splat(150.0);
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(center.x - 105.0, 10.0, center.y - 105.0),
			Vec3::new(center.x + 105.0, 26.0, center.y + 105.0),
		));
		let noise = NoiseParams { seed: 29, ..NoiseParams::default() };
		let first = SuburbanHomes::fit(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("first neighborhood did not fit"))?;
		let second = SuburbanHomes::fit(cell, &confines, noise)
			.ok_or_else(|| anyhow::anyhow!("second neighborhood did not fit"))?;
		let first_buildings: Vec<_> = first.buildings().collect();
		let second_buildings: Vec<_> = second.buildings().collect();
		assert_eq!(first_buildings.len(), second_buildings.len());
		for (a, b) in first_buildings.iter().zip(&second_buildings) {
			assert_eq!(a.center_xz, b.center_xz);
			assert_eq!(a.yaw, b.yaw);
			assert_eq!(a.footprint, b.footprint);
			assert_eq!(shepherds_finish(a)?, shepherds_finish(b)?);
		}
		assert!(first.homes.len() >= 7);
		assert!(!first.secondary_buildings.is_empty());
		assert!(first
			.secondary_buildings
			.iter()
			.all(|building| matches!(&building.building, ShepherdsBuilding::Hut(_))));

		let finishes: Vec<_> =
			first.homes.iter().map(shepherds_finish).collect::<anyhow::Result<_>>()?;
		let distinct = finishes
			.iter()
			.enumerate()
			.filter(|(index, finish)| finishes[..*index].iter().all(|earlier| *earlier != **finish))
			.count();
		assert!(distinct > 1, "representative neighborhood should vary house finishes");

		let bounds = Aabb2d {
			min: Vec2::new(first.bounds.min.x, first.bounds.min.z),
			max: Vec2::new(first.bounds.max.x, first.bounds.max.z),
		};
		let plan_bounds = |building: &ShepherdsVillageBuilding| {
			let half =
				yawed_plan_aabb_extent(building.footprint.x, building.footprint.y, building.yaw)
					* 0.5;
			Bounds2 { min: building.center_xz - half, max: building.center_xz + half }
		};
		for (index, building) in first_buildings.iter().enumerate() {
			let footprint = plan_bounds(building);
			assert!(bounds_contains(bounds, footprint));
			for other in &first_buildings[..index] {
				assert!(!bounds_intersect(footprint, plan_bounds(other)));
			}
		}
		Ok(())
	}
}
