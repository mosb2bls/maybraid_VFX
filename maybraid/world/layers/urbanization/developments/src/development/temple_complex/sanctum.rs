//! Seeded ornamental landmarks used at the center of a temple complex.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::{
	BuildingComponents, BuildingStructuralLodProbe, FloorNode, JointNode, Layers, PanelNode,
	PartitionNode, StairNode,
};
use buildings::{
	Confines, FillableRegions, Fit, FitError, Openings, RectFloor, RectFloorParams, RectFloorSlab,
	Trazaloid, TrazaloidParams, TrazaloidSlab,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::development::ring_fort::{RingFortKeep, TOWER_STOREY_HEIGHT};
use crate::BuildingFootprint;

const MIN_PLAN_METERS: f32 = 16.0;
const MIN_HEIGHT_METERS: f32 = 16.0;

/// Components and finish stamps shared by each authored sanctum recipe.
#[derive(Debug, Clone, PartialEq)]
pub struct TempleSanctumComponents {
	pub bounds: Aabb3d,
	pub podium: Option<RectFloor>,
	pub keeps: Vec<RingFortKeep>,
	pub ornaments: Vec<Trazaloid>,
	pub footprints: Vec<Aabb2d>,
	pub wall_material: Option<MaterialRef>,
	pub ornament_material: Option<MaterialRef>,
}

/// One of three deterministic, genuinely different temple landmark compositions.
#[derive(Debug, Clone, PartialEq)]
pub enum TempleSanctum {
	/// A circular spiral keep carrying an oversized trazaloid crown.
	TrazaloidCrown(TempleSanctumComponents),
	/// Two circular spiral keeps rise from a shared rectangular podium.
	TwinSpiralPodium(TempleSanctumComponents),
	/// A tall stacked trazaloid keep tapers into an obelisk.
	TaperedObelisk(TempleSanctumComponents),
}

impl TempleSanctum {
	pub const RECIPE_COUNT: usize = 3;

	/// Stable recipe lane selected from a generation seed.
	pub fn recipe_index(&self) -> usize {
		match self {
			Self::TrazaloidCrown(_) => 0,
			Self::TwinSpiralPodium(_) => 1,
			Self::TaperedObelisk(_) => 2,
		}
	}

	pub fn bounds(&self) -> Aabb3d {
		self.components().bounds
	}

	pub fn components(&self) -> &TempleSanctumComponents {
		match self {
			Self::TrazaloidCrown(components)
			| Self::TwinSpiralPodium(components)
			| Self::TaperedObelisk(components) => components,
		}
	}

	fn components_mut(&mut self) -> &mut TempleSanctumComponents {
		match self {
			Self::TrazaloidCrown(components)
			| Self::TwinSpiralPodium(components)
			| Self::TaperedObelisk(components) => components,
		}
	}

	/// Stamp the structural wall and contrasting ornamental/cap finish.
	pub fn with_finish(mut self, wall: MaterialRef, ornament: MaterialRef) -> Self {
		let components = self.components_mut();
		components.wall_material = Some(wall);
		components.ornament_material = Some(ornament);
		self
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.components_mut().wall_material = Some(wall);
		self
	}

	fn validate_confines(confines: &Confines) -> Result<(Vec2, Vec2, f32, f32), FitError> {
		let extent = confines.footprint();
		let height = confines.bounds.max.y - confines.bounds.min.y;
		if extent.x < MIN_PLAN_METERS || extent.y < MIN_PLAN_METERS || height < MIN_HEIGHT_METERS {
			return Err(FitError::TooSmall { reason: "temple_sanctum" });
		}
		Ok((confines.center_xz(), extent, confines.bounds.min.y, height))
	}

	fn trazaloid_crown(confines: &Confines) -> Result<Self, FitError> {
		let (center, extent, y, height) = Self::validate_confines(confines)?;
		let side = extent.x.min(extent.y);
		let radius = side * 0.24;
		let crown_height = (height * 0.18).clamp(5.2, 7.0);
		let floors = (((height - crown_height) / TOWER_STOREY_HEIGHT).floor() as usize).max(3);
		let shaft_height = floors as f32 * TOWER_STOREY_HEIGHT;
		let origin = Vec3::new(center.x, y, center.y);
		let crown_foot = radius * 1.8;
		let crown_origin = origin + Vec3::Y * shaft_height;
		let crown = TrazaloidParams {
			origin: crown_origin,
			footprint: Vec2::splat(crown_foot),
			ridge: Vec2::splat(crown_foot * 0.48),
			lower_height: crown_height * 0.48,
			upper_height: crown_height * 0.38,
			band_vertical_offset: crown_height * 0.14,
			openings: Openings::new(),
			floor: TrazaloidSlab::Solid,
			ceiling: TrazaloidSlab::Solid,
			..TrazaloidParams::default()
		}
		.build();
		let half = crown_foot.max(radius * 2.0) * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, y, center.y - half),
			Vec3::new(center.x + half, crown_origin.y + crown_height, center.y + half),
		);
		Ok(Self::TrazaloidCrown(TempleSanctumComponents {
			bounds,
			podium: None,
			keeps: vec![RingFortKeep::circular(origin, radius, floors)],
			ornaments: vec![crown],
			footprints: vec![Aabb2d::new(center, Vec2::splat(half))],
			wall_material: None,
			ornament_material: None,
		}))
	}

	fn twin_spiral_podium(confines: &Confines) -> Result<Self, FitError> {
		let (center, extent, y, height) = Self::validate_confines(confines)?;
		let side = extent.x.min(extent.y);
		let podium_height = 4.0_f32.min(height * 0.25);
		let radius = side * 0.14;
		let floors = (((height - podium_height) / TOWER_STOREY_HEIGHT).floor() as usize).max(3);
		let tower_y = y + podium_height;
		let along_x = extent.x >= extent.y;
		let offset =
			if along_x { Vec2::new(extent.x * 0.22, 0.0) } else { Vec2::new(0.0, extent.y * 0.22) };
		let podium_footprint = extent * 0.9;
		let podium =
			RectFloorParams::new(Vec3::new(center.x, y, center.y), podium_footprint, podium_height)
				.floor(RectFloorSlab::Solid)
				.ceiling(RectFloorSlab::Solid)
				.build();
		let tower_centers = [center - offset, center + offset];
		let keeps = tower_centers
			.into_iter()
			.map(|tower_center| {
				RingFortKeep::circular(
					Vec3::new(tower_center.x, tower_y, tower_center.y),
					radius,
					floors,
				)
			})
			.collect();
		let half = podium_footprint * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half.x, y, center.y - half.y),
			Vec3::new(
				center.x + half.x,
				tower_y + floors as f32 * TOWER_STOREY_HEIGHT,
				center.y + half.y,
			),
		);
		let mut footprints = vec![Aabb2d::new(center, half)];
		footprints.extend(
			tower_centers
				.into_iter()
				.map(|tower_center| Aabb2d::new(tower_center, Vec2::splat(radius))),
		);
		Ok(Self::TwinSpiralPodium(TempleSanctumComponents {
			bounds,
			podium: Some(podium),
			keeps,
			ornaments: Vec::new(),
			footprints,
			wall_material: None,
			ornament_material: None,
		}))
	}

	fn tapered_obelisk(confines: &Confines) -> Result<Self, FitError> {
		let (center, extent, y, height) = Self::validate_confines(confines)?;
		let foot = extent.x.min(extent.y) * 0.68;
		let floors = ((height / TOWER_STOREY_HEIGHT).floor() as usize).max(4);
		let origin = Vec3::new(center.x, y, center.y);
		let half = foot * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, y, center.y - half),
			Vec3::new(center.x + half, y + floors as f32 * TOWER_STOREY_HEIGHT, center.y + half),
		);
		Ok(Self::TaperedObelisk(TempleSanctumComponents {
			bounds,
			podium: None,
			keeps: vec![RingFortKeep::trazaloid(origin, foot, floors, (1.0, 1.0))],
			ornaments: Vec::new(),
			footprints: vec![Aabb2d::new(center, Vec2::splat(half))],
			wall_material: None,
			ornament_material: None,
		}))
	}

	fn extend_keep_stairs(
		components: &TempleSanctumComponents,
		level: LodSceneLevel,
		out: &mut Layers<StairNode>,
	) {
		if !matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
			return;
		}
		for keep in &components.keeps {
			for stairwell in keep.stairwells() {
				out.extend(stairwell.stair_nodes_for_level(level));
			}
		}
	}
}

impl Fit for TempleSanctum {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let sanctum = match noise.seed.rem_euclid(Self::RECIPE_COUNT as i32) {
			0 => Self::trazaloid_crown(confines)?,
			1 => Self::twin_spiral_podium(confines)?,
			_ => Self::tapered_obelisk(confines)?,
		};
		Ok((sanctum, FillableRegions::empty()))
	}
}

impl BuildingFootprint for TempleSanctum {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		self.components().footprints.clone()
	}
}

impl BuildingComponents for TempleSanctum {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let components = self.components();
		let mut structure = Layers::new();
		if let Some(podium) = &components.podium {
			structure.extend(podium.panel_nodes_for_level(level));
		}
		for keep in &components.keeps {
			structure.extend(keep.panel_nodes_for_level(level));
			for stairwell in keep.stairwells() {
				structure.extend(stairwell.panel_nodes_for_level(level));
			}
		}
		if let Some(material) = &components.wall_material {
			structure = structure.with_material(material.clone());
		}
		let mut ornaments = Layers::new();
		for ornament in &components.ornaments {
			ornaments.extend(ornament.panel_nodes_for_level(level));
		}
		if let Some(material) =
			components.ornament_material.as_ref().or(components.wall_material.as_ref())
		{
			ornaments = ornaments.with_material(material.clone());
		}
		structure.extend(ornaments);
		structure
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let components = self.components();
		let mut out = Layers::new();
		for keep in &components.keeps {
			out.extend(keep.partition_nodes_for_level(level));
		}
		if let Some(material) = &components.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		let components = self.components();
		let mut out = Layers::new();
		for keep in &components.keeps {
			out.extend(keep.floor_nodes_for_level(level));
		}
		out
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		let mut out = Layers::new();
		Self::extend_keep_stairs(self.components(), level, &mut out);
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let components = self.components();
		let mut out = Layers::new();
		if let Some(podium) = &components.podium {
			out.extend(podium.joint_nodes_for_level(level));
		}
		for keep in &components.keeps {
			out.extend(keep.joint_nodes_for_level(level));
			for stairwell in keep.stairwells() {
				out.extend(stairwell.joint_nodes_for_level(level));
			}
		}
		for ornament in &components.ornaments {
			out.extend(ornament.joint_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let components = self.components();
		let wall = components.wall_material.clone();
		let ornament = components.ornament_material.clone().or_else(|| wall.clone());
		let mut volumes = Vec::new();
		if let Some(podium) = &components.podium {
			if let Some(probe) = podium.structural_lod() {
				volumes
					.extend(probe.volumes.into_iter().map(|v| v.with_material_opt(wall.clone())));
			}
		}
		for keep in &components.keeps {
			match keep {
				RingFortKeep::Circular(keep) => {
					if let Some(probe) = keep.shell.structural_lod() {
						volumes.extend(
							probe.volumes.into_iter().map(|v| v.with_material_opt(wall.clone())),
						);
					}
				}
				RingFortKeep::Trazaloid(keep) => {
					if let Some(probe) = keep.shell.structural_lod() {
						volumes.extend(
							probe.volumes.into_iter().map(|v| v.with_material_opt(wall.clone())),
						);
					}
				}
			}
		}
		for shell in &components.ornaments {
			if let Some(probe) = shell.structural_lod() {
				volumes.extend(
					probe.volumes.into_iter().map(|v| v.with_material_opt(ornament.clone())),
				);
			}
		}
		if volumes.is_empty() {
			None
		} else {
			Some(BuildingStructuralLodProbe::from_volumes(volumes))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-18.0, 2.0, -18.0),
			Vec3::new(18.0, 50.0, 18.0),
		))
	}

	#[test]
	fn all_recipes_fit_and_present_authored_components() -> anyhow::Result<()> {
		for seed in 0..TempleSanctum::RECIPE_COUNT {
			let (sanctum, _) = TempleSanctum::fit_to_confines(
				&confines(),
				NoiseParams { seed: seed as i32, ..NoiseParams::default() },
			)?;
			assert_eq!(sanctum.recipe_index(), seed);
			let footprints = sanctum.footprint_rects();
			assert!(!footprints.is_empty());
			for footprint in footprints {
				assert!(footprint.min.cmpge(Vec2::splat(-18.0)).all());
				assert!(footprint.max.cmple(Vec2::splat(18.0)).all());
			}
			assert!(
				!sanctum.panel_nodes_for_level(LodSceneLevel::High).is_empty()
					|| !sanctum.partition_nodes_for_level(LodSceneLevel::High).is_empty()
			);
			let bounds = sanctum.bounds();
			let parent = confines().bounds;
			assert!(bounds.min.cmpge(parent.min).all());
			assert!(bounds.max.cmple(parent.max + 1e-3).all());

			let wall = MaterialRef::named("temple-wall");
			let ornament = MaterialRef::named("temple-ornament");
			let finished = sanctum.with_finish(wall.clone(), ornament.clone());
			let panels: Vec<_> = finished.panel_nodes_for_level(LodSceneLevel::High).flatten();
			let partitions: Vec<_> =
				finished.partition_nodes_for_level(LodSceneLevel::High).flatten();
			assert!(panels.iter().all(|node| node.material.is_some()));
			assert!(partitions.iter().all(|node| node.material.as_ref() == Some(&wall)));
			if seed == 0 {
				assert!(panels.iter().any(|node| node.material.as_ref() == Some(&ornament)));
			}
		}
		Ok(())
	}

	#[test]
	fn recipe_selection_and_materials_are_deterministic() -> anyhow::Result<()> {
		let noise = NoiseParams { seed: 4, ..NoiseParams::default() };
		let (a, _) = TempleSanctum::fit_to_confines(&confines(), noise)?;
		let (b, _) = TempleSanctum::fit_to_confines(&confines(), noise)?;
		assert_eq!(a, b);

		Ok(())
	}
}
