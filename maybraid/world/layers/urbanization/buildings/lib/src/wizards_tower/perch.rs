//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Same treatment as a regular storey for now: [`crate::arcs::PortalRingWall`] + squared floor.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use building_components::floors::FloorNode;
use building_components::partitions::PartitionStyle;
use building_components::scene_children;
use building_components::{
	append_flattened_component_scenes, BuildingComponents, Layers, PartitionNode,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::arcs::{portal_ring_wall, PortalRingParams, PortalRingWall};
use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::must_assign_cardinal_portals;
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale).
	pub storey_height: f32,
	pub ring_wall: PortalRingWall,
	pub floor_caps: [FloorNode; 4],
	pub floor_rects: [FloorNode; 4],
	pub wall_material: Option<MaterialRef>,
}

impl WizardsTowerPerch {
	/// Build from this perch's subsetted constraints, storey height, and portal noise.
	pub fn new(
		constraints: CellConstraints,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let spire_half = SPIRE_HALF_FRAC * radius;
		let (floor_caps, floor_rects) =
			squared_floor_with_spire_hole(center_xz, radius, spire_half);

		let ring_wall = portal_ring_wall(PortalRingParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees: 360.0,
			start_yaw: 0.0,
			must_assign: must_assign_cardinal_portals(),
			must_not_assign: vec![],
			portal_noise,
			optional_portals: (0, 2),
			style: PartitionStyle::RoughStonework,
		});

		Self { storey_height, ring_wall, floor_caps, floor_rects, wall_material: None, constraints }
	}

	pub fn with_wall_material(mut self, material: MaterialRef) -> Self {
		self.wall_material = Some(material);
		self
	}

	pub(crate) fn emit_external_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		append_flattened_component_scenes(self, lod_ref, LodSceneLevel::Medium, children);
	}

	fn is_structure_level(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
	}
}

impl BuildingComponents for WizardsTowerPerch {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if Self::is_structure_level(level) {
			let mut out = self.ring_wall.sweep.partition_nodes_for_level(level);
			if let Some(material) = &self.wall_material {
				out = out.with_material(material.clone());
			}
			out
		} else {
			Layers::new()
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		if !Self::is_structure_level(level) {
			return Layers::new();
		}
		// Exterior deck, same band as the ring wall. See `WizardsTowerFloor`.
		Layers::from_free(self.floor_caps.iter().chain(self.floor_rects.iter()).cloned().collect())
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::scene::LodSceneStatus {
		lod::scene::LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		scene_children(children)
	}
}
