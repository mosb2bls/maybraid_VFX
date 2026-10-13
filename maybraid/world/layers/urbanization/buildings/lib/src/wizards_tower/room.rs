//! Voxel halfspace / room fill around the Wizard's Tower spire.
//!
//! Geometry: one linear partition on the spire-facing edge and a rectangular
//! floor slab. Doors / stairs are omitted for now (empty scenes).

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::floors::{Floor, FloorNode};
use building_components::partitions::{
	wall_placement_from_centered, Partition, PartitionNode, DEFAULT_THICK,
};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::wizards_tower::floor_fill::{FLOOR_SLAB_Y_SCALE, INSCRIBED_HALF_FRAC};
use crate::CellConstraints;

/// A bounded room / voxel-halfspace child of a tower floor.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerRoom {
	pub constraints: CellConstraints,
	pub partition: PartitionNode,
	pub floor: FloorNode,
	pub wall_material: Option<MaterialRef>,
}

impl WizardsTowerRoom {
	/// Build from this room's subsetted constraints.
	pub fn new(constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let size = constraints.aabb.max - constraints.aabb.min;
		let yaw = if size.x >= size.z { std::f32::consts::FRAC_PI_2 } else { 0.0 };
		let half_len = size.x.max(size.z) * 0.5;
		let height = size.y.max(1e-4);
		let width = size.x.max(1e-4);
		let depth = size.z.max(1e-4);
		// Panel-space lower-left; FloorNode remaps to the centered floor kit.
		let floor_origin =
			Vec3::new(constraints.aabb.min.x, constraints.aabb.min.y, constraints.aabb.min.z);

		Self {
			partition: PartitionNode::rough_stone(
				Partition::linear(),
				wall_placement_from_centered(center_xz, yaw, half_len, height, DEFAULT_THICK),
			),
			floor: FloorNode::rough_stone(
				Floor::rectangle(),
				Placement::new(floor_origin, 0.0).with_scale(Vec3::new(
					width,
					FLOOR_SLAB_Y_SCALE,
					depth,
				)),
			),
			wall_material: None,
			constraints,
		}
	}

	/// Four non-overlapping corner rooms between the inscribed-square edge and central shaft.
	pub fn quadrants(
		parent: &CellConstraints,
		radius: f32,
		spire_half: f32,
	) -> [WizardsTowerRoom; 4] {
		let center = (parent.aabb.min + parent.aabb.max) * 0.5;
		let outer = INSCRIBED_HALF_FRAC * radius;
		let shaft = spire_half.min(outer * 0.9);
		let clearance = DEFAULT_THICK.max(0.05);
		let x = [
			(center.x - outer + clearance, center.x - shaft - clearance),
			(center.x + shaft + clearance, center.x + outer - clearance),
		];
		let z = [
			(center.z - outer + clearance, center.z - shaft - clearance),
			(center.z + shaft + clearance, center.z + outer - clearance),
		];
		let room = |xi: usize, zi: usize| {
			let bounds = Aabb3d::from_min_max(
				Vec3::new(x[xi].0, parent.aabb.min.y, z[zi].0),
				Vec3::new(x[xi].1, parent.aabb.max.y, z[zi].1),
			);
			let constraints =
				parent.subset(bounds).unwrap_or_else(|_| CellConstraints::cell_owned(bounds));
			Self::new(constraints)
		};
		[room(0, 0), room(1, 0), room(0, 1), room(1, 1)]
	}

	pub fn with_wall_material(mut self, material: MaterialRef) -> Self {
		self.wall_material = Some(material);
		self
	}
}

impl BuildingComponents for WizardsTowerRoom {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if !matches!(level, LodSceneLevel::High) {
			return Layers::new();
		}
		let mut out = Layers::from_free(vec![self.partition.clone()]);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		if matches!(level, LodSceneLevel::High) {
			Layers::from_free(vec![self.floor.clone()])
		} else {
			Layers::new()
		}
	}
}
