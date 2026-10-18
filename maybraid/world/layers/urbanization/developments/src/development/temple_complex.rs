//! [`TempleComplex`]: a sanctum campus on one terrace.

mod sanctum;

pub use sanctum::{TempleSanctum, TempleSanctumComponents};

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use buildings::{Confines, Fit, Openings};
use material_ref::MaterialRef;
use procedural_common::{NoiseParams, SeededHash};

use super::ring_fort::RING_FORT_MAX_FOOTPRINT;
use super::root_hash;
use super::shepherds_village::fit::{fit_shepherds_building_for_role, ShepherdsBuildingKind};
use super::shepherds_village::ShepherdsVillageBuilding;
use crate::{
	DevelopmentFinish, DevelopmentFinishRole, PlacedBuilding, Terrace, TerraceDevelopment,
	TerraceEnvelope,
};

/// Axial halls around a taller central shrine.
#[derive(Debug, Clone, PartialEq)]
pub struct TempleComplex {
	pub bounds: Aabb3d,
	pub halls: Vec<ShepherdsVillageBuilding>,
	pub sanctum: PlacedBuilding<TempleSanctum>,
}

impl TerraceDevelopment for TempleComplex {
	fn envelope() -> TerraceEnvelope {
		TerraceEnvelope {
			min_footprint: 130.0,
			max_footprint: RING_FORT_MAX_FOOTPRINT.min(190.0),
			min_height: 48.0,
			max_height: 64.0,
			rotates: false,
		}
	}

	fn finish(hash: SeededHash) -> DevelopmentFinish {
		DevelopmentFinish::pick_for_role(hash, DevelopmentFinishRole::Temple, false)
	}

	fn fit_terrace(terrace: &Terrace) -> Option<Self> {
		let finish = terrace.finish.clone();
		Some(
			Self::fit(terrace.cell, &terrace.confines(), terrace.noise())?
				.with_finish(finish.wall, finish.roof),
		)
	}
}

impl TempleComplex {
	pub fn with_finish(mut self, wall: MaterialRef, ornament: MaterialRef) -> Self {
		self.sanctum.building = self.sanctum.building.with_finish(wall, ornament);
		self
	}

	pub fn with_landmark_material(mut self, wall: MaterialRef) -> Self {
		self.sanctum.building = self.sanctum.building.with_wall_material(wall);
		self
	}

	/// Four axial halls around a central sanctum inside `confines` on `cell`.
	pub fn fit(cell: Aabb3d, confines: &Confines, noise: NoiseParams) -> Option<Self> {
		let center = confines.center_xz();
		let y = confines.bounds.min.y;
		let extent = confines.footprint();
		let root = root_hash(cell, noise);
		let mut halls = Vec::new();
		for (index, offset) in [
			Vec2::new(0.0, -extent.y * 0.31),
			Vec2::new(0.0, extent.y * 0.31),
			Vec2::new(-extent.x * 0.31, 0.0),
			Vec2::new(extent.x * 0.31, 0.0),
		]
		.into_iter()
		.enumerate()
		{
			let along_x = offset.x.abs() < offset.y.abs();
			let footprint = if along_x { Vec2::new(30.0, 18.0) } else { Vec2::new(18.0, 30.0) };
			let yaw = if along_x { 0.0 } else { std::f32::consts::FRAC_PI_2 };
			let hash = SeededHash::new(root.seed.wrapping_add(index as u32 * 97 + 1));
			let mut local_noise = noise;
			local_noise.seed = noise.seed.wrapping_add(index as i32 * 97);
			if let Some(hall) = fit_shepherds_building_for_role(
				ShepherdsBuildingKind::House,
				center + offset,
				yaw,
				footprint,
				y,
				hash,
				local_noise,
				DevelopmentFinishRole::Temple,
			) {
				halls.push(hall);
			}
		}

		let sanctum_footprint =
			Vec2::splat(extent.x.min(extent.y).mul_add(0.20, 0.0).clamp(28.0, 36.0));
		let sanctum_bounds = Aabb3d::from_min_max(
			Vec3::new(
				center.x - sanctum_footprint.x * 0.5,
				y,
				center.y - sanctum_footprint.y * 0.5,
			),
			Vec3::new(
				center.x + sanctum_footprint.x * 0.5,
				(y + 44.0).min(confines.bounds.max.y),
				center.y + sanctum_footprint.y * 0.5,
			),
		);
		let sanctum_confines = Confines::new(sanctum_bounds, 0.0, Openings::new());
		let (sanctum_building, _) =
			TempleSanctum::fit_to_confines(&sanctum_confines, noise).ok()?;
		let sanctum = PlacedBuilding {
			center_xz: center,
			yaw: 0.0,
			footprint: sanctum_footprint,
			ground_height: y,
			building: sanctum_building,
		};
		Some(Self { bounds: confines.bounds, halls, sanctum })
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn temple_emits_four_halls_and_a_sanctum() -> anyhow::Result<()> {
		let cell = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(65.0, 10.0, 65.0),
			Vec3::new(235.0, 70.0, 235.0),
		));
		let temple = TempleComplex::fit(cell, &confines, NoiseParams::default())
			.ok_or_else(|| anyhow::anyhow!("temple did not fit"))?;
		assert_eq!(temple.halls.len(), 4);
		assert!(temple.sanctum.building.recipe_index() < TempleSanctum::RECIPE_COUNT);
		Ok(())
	}
}
