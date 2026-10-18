//! The stacked circular column of the Wizard's Tower.

use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::floors::FloorNode;
use building_components::scene_children;
use building_components::stairs::StairNode;
use building_components::{BuildingComponents, Layers, PartitionNode};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::wizards_tower::floor_fill::WALL_HEIGHT_METERS;
use crate::wizards_tower::tower_lod::TowerLodFootprint;
use crate::wizards_tower::{WizardsTowerFloor, WizardsTowerPerch};
use crate::CellConstraints;

/// Vertical stack of tower floors capped by a perch.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerColumn {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale; floor spacing).
	pub storey_height: f32,
	pub floors: Vec<WizardsTowerFloor>,
	pub perch: WizardsTowerPerch,
}

impl WizardsTowerColumn {
	/// Build from the tower footprint constraints, floor count, storey height, and
	/// a base portal [`NoiseParams`] (per-storey seeds are derived from this).
	///
	/// Each regular storey occupies \([y_i, y_i + h)\) with \(h =\) `storey_height`.
	/// The perch sits in the next slab of the same height. Footprint \(XZ\) and base
	/// \(Y\) come from `tower_constraints`; the parent AABB's max \(Y\) is ignored.
	pub fn new(
		tower_constraints: &CellConstraints,
		floor_count: u32,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let floor_count = floor_count.max(1);

		let min_y = tower_constraints.aabb.min.y;
		let floors_top = min_y + floor_count as f32 * storey_height;
		let perch_top = floors_top + storey_height;

		// Rebuild write AABB to match the stacked height (XZ from the footprint).
		let footprint = Aabb3d::from_min_max(
			Vec3::new(tower_constraints.aabb.min.x, min_y, tower_constraints.aabb.min.z),
			Vec3::new(tower_constraints.aabb.max.x, perch_top, tower_constraints.aabb.max.z),
		);
		let constraints = tower_constraints
			.subset(footprint)
			.unwrap_or_else(|_| CellConstraints::cell_owned(footprint));

		let floors = (0..floor_count)
			.map(|i| {
				let y0 = min_y + i as f32 * storey_height;
				let y1 = y0 + storey_height;
				let floor_aabb = Self::vertical_slab(&constraints.aabb, y0, y1);
				let floor_constraints = constraints
					.subset(floor_aabb)
					.unwrap_or_else(|_| CellConstraints::cell_owned(floor_aabb));
				let mut floor_noise = portal_noise;
				floor_noise.seed = portal_noise.seed.wrapping_add(i as i32 * 97);
				WizardsTowerFloor::new(floor_constraints, storey_height, floor_noise)
			})
			.collect();

		let perch_aabb = Self::vertical_slab(&constraints.aabb, floors_top, perch_top);
		let perch_constraints = constraints
			.subset(perch_aabb)
			.unwrap_or_else(|_| CellConstraints::cell_owned(perch_aabb));
		let mut perch_noise = portal_noise;
		perch_noise.seed = portal_noise.seed.wrapping_add(floor_count as i32 * 97 + 13);
		let perch = WizardsTowerPerch::new(perch_constraints, storey_height, perch_noise);

		Self { constraints, storey_height, floors, perch }
	}

	/// Same as [`Self::new`] with [`WALL_HEIGHT_METERS`].
	pub fn with_default_storey_height(
		tower_constraints: &CellConstraints,
		floor_count: u32,
		portal_noise: NoiseParams,
	) -> Self {
		Self::new(tower_constraints, floor_count, WALL_HEIGHT_METERS, portal_noise)
	}

	pub fn with_materials(mut self, wall: MaterialRef, room: MaterialRef) -> Self {
		self.floors = self
			.floors
			.into_iter()
			.map(|floor| floor.with_materials(wall.clone(), room.clone()))
			.collect();
		self.perch = self.perch.with_wall_material(wall);
		self
	}

	fn vertical_slab(parent: &Aabb3d, y_min: f32, y_max: f32) -> Aabb3d {
		Aabb3d::from_min_max(
			Vec3::new(parent.min.x, y_min, parent.min.z),
			Vec3::new(parent.max.x, y_max, parent.max.z),
		)
	}

	/// One vertical capsule for the continuous spire shaft (all regular floors).
	///
	/// Radius matches the centered spire hole; reveal uses [`INTERNAL_REVEAL_FACTOR`].
	pub fn spire_confine_capsule(&self) -> building_components::ParentConfines {
		use crate::wizards_tower::floor_fill::SPIRE_HALF_FRAC;
		use building_components::ParentConfines;

		let aabb = &self.constraints.aabb;
		let c = (aabb.min + aabb.max) * 0.5;
		// Spire runs through regular floors only (perch has no stair run).
		let floors_top = if self.floors.is_empty() {
			aabb.min.y
		} else {
			aabb.min.y + self.floors.len() as f32 * self.storey_height
		};
		let footprint_r = 0.5 * (aabb.max.x - aabb.min.x).min(aabb.max.z - aabb.min.z);
		let radius = (SPIRE_HALF_FRAC * footprint_r).max(1e-4);
		ParentConfines::capsule(
			Vec3::new(c.x, aabb.min.y, c.z),
			Vec3::new(c.x, floors_top, c.z),
			radius,
		)
	}
}

impl BuildingComponents for WizardsTowerColumn {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = Layers::new();
		for f in &self.floors {
			out.extend(f.partition_nodes_for_level(level));
		}
		out.extend(self.perch.partition_nodes_for_level(level));
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		let mut out = Layers::new();
		for f in &self.floors {
			out.extend(f.floor_nodes_for_level(level));
		}
		out.extend(self.perch.floor_nodes_for_level(level));
		out
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		if !matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
			return Layers::new();
		}
		let spire_confines = self.spire_confine_capsule();
		Layers::from_free(
			self.floors
				.iter()
				.map(|f| f.arc_spire.stairs.clone().with_confines(spire_confines))
				.collect(),
		)
	}
}

impl TowerLodFootprint for WizardsTowerColumn {
	fn lod_aabb(&self) -> &Aabb3d {
		&self.constraints.aabb
	}
}

impl LodScene for WizardsTowerColumn {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::scene::LodSceneStatus {
		lod::scene::LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = self
			.floors
			.iter()
			.map(|floor| Box::new(floor.scene_with_lod(lod_ref)) as Box<dyn Scene>)
			.collect();
		children.push(Box::new(self.perch.scene_with_lod(lod_ref)));
		scene_children(children)
	}
}
