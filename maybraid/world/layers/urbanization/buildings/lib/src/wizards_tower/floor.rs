//! A floor of the Wizard's Tower.
//!
//! Geometry: outer [`crate::arcs::PortalRingWall`] (clipped circular sweep with door/window
//! portals), squared-off floor with a centered spire hole, and a crate-level
//! [`crate::ArcSpire`] tread run inside the spire square that rises one storey. Each storey
//! also carries a lantern-like point light (mesh TBD).

use bevy::prelude::{Color, PointLight, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use building_components::floors::FloorNode;
use building_components::partitions::PartitionStyle;
use building_components::scene_children;
use building_components::stairs::{SpiralStair, StairNode};
use building_components::{
	append_flattened_component_scenes, confined_scene, BuildingComponents, Layers, ParentConfines,
	PartitionNode,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::arc_spire::{uniform_storey_bindings, ArcSpire, ArcSpireParams, FitTolerance};
use crate::arcs::{portal_ring_wall, PortalRingParams, PortalRingWall};
use crate::wizards_tower::floor_fill::{squared_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::must_assign_cardinal_portals;
use crate::wizards_tower::WizardsTowerRoom;
use crate::CellConstraints;

/// One storey of the circular tower.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerFloor {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer wall \(Y\) scale).
	pub storey_height: f32,
	/// Outer arc wall with portals.
	pub ring_wall: PortalRingWall,
	/// Four circle−inscribed-square caps that square off the circular footprint.
	pub floor_caps: [FloorNode; 4],
	/// Rectangular slabs filling the inscribed square around the spire hole.
	pub floor_rects: [FloorNode; 4],
	/// Circular tread spire inside the spire square, fitted to storey \(Y\) bindings.
	pub arc_spire: ArcSpire,
	/// Four usable corner rooms, clear of the central stair shaft.
	pub rooms: [WizardsTowerRoom; 4],
	/// Warm lantern point light hanging over the usable floor (no mesh yet).
	pub lantern: Vec3,
	pub wall_material: Option<MaterialRef>,
	pub room_material: Option<MaterialRef>,
}

impl WizardsTowerFloor {
	/// Build from this floor's subsetted constraints, storey height, and portal noise
	/// (seeded per storey).
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

		// Spiral stays inside the centered spire square (outer tread edge ≤ spire_half).
		let tread_width = spire_half * 0.45;
		let stair_radius = (spire_half - 0.5 * tread_width).max(1e-4);
		let tread_depth = tread_width * 0.55;
		let target_tread_height = SpiralStair::DEFAULT_TREAD_HEIGHT;

		// Hang over the floor ring, clear of the spire stairs (~chest / lantern height).
		let lantern = Vec3::new(
			center.x + 0.55 * radius,
			constraints.aabb.min.y + storey_height * 0.65,
			center.z,
		);

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

		let arc_spire = ArcSpire::new(ArcSpireParams {
			center_xz,
			radius: stair_radius,
			tread_width,
			tread_depth,
			target_tread_height,
			y_bindings: uniform_storey_bindings(center_xz.y, storey_height, target_tread_height),
			fit_tolerance: FitTolerance::default(),
			turns: 1.0,
		});
		let rooms = WizardsTowerRoom::quadrants(&constraints, radius, spire_half);

		Self {
			storey_height,
			ring_wall,
			floor_caps,
			floor_rects,
			arc_spire,
			rooms,
			lantern,
			wall_material: None,
			room_material: None,
			constraints,
		}
	}

	pub fn with_materials(mut self, wall: MaterialRef, room: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self.room_material = Some(room.clone());
		self.rooms = self.rooms.map(|authored_room| authored_room.with_wall_material(room.clone()));
		self
	}

	pub(crate) fn emit_external_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		append_flattened_component_scenes(self, lod_ref, LodSceneLevel::Medium, children);
	}

	pub(crate) fn emit_internal_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		// The walking deck is exterior (see `floor_nodes_for_level`). Only rooms
		// and the lantern wait for the close reveal.
		let confines =
			ParentConfines::internal(self.storey_confine_center(), self.storey_confine_radius());
		for room in &self.rooms {
			for node in room.floor_nodes_for_level(LodSceneLevel::High).flatten() {
				children.push(Box::new(node.scene_with_level(lod_ref, LodSceneLevel::High)));
			}
			for node in room.partition_nodes_for_level(LodSceneLevel::High).flatten() {
				children.push(Box::new(node.scene_with_level(lod_ref, LodSceneLevel::High)));
			}
		}
		children.push(Box::new(confined_scene(confines, self.lantern_scene())));
	}

	fn storey_confine_center(&self) -> Vec3 {
		let aabb = &self.constraints.aabb;
		Vec3::from((aabb.min + aabb.max) * 0.5)
	}

	fn storey_confine_radius(&self) -> f32 {
		let aabb = &self.constraints.aabb;
		let extent = aabb.max - aabb.min;
		(0.5 * extent.x.min(extent.z)).max(1e-4)
	}

	/// Capsule for this storey alone (standalone floor present).
	fn storey_spire_capsule(&self) -> ParentConfines {
		let aabb = &self.constraints.aabb;
		let c = (aabb.min + aabb.max) * 0.5;
		let r = (SPIRE_HALF_FRAC * self.storey_confine_radius()).max(1e-4);
		ParentConfines::capsule(Vec3::new(c.x, aabb.min.y, c.z), Vec3::new(c.x, aabb.max.y, c.z), r)
	}

	fn lantern_scene(&self) -> impl Scene + 'static {
		let range = (self.storey_height * 2.5).max(4.0);
		let transform = Transform::from_translation(self.lantern);
		bsn! {
			PointLight {
				color: Color::srgb(1.0, 0.72, 0.42),
				intensity: 2800.0,
				range: {range},
				shadow_maps_enabled: false,
			}
			template_value(transform)
			Visibility::default()
		}
	}

	fn is_detail_level(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High)
	}

	fn is_structure_level(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
	}
}

impl BuildingComponents for WizardsTowerFloor {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if !Self::is_structure_level(level) {
			return Layers::new();
		}
		let mut out = self.ring_wall.sweep.partition_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		if Self::is_detail_level(level) {
			for room in &self.rooms {
				out.extend(room.partition_nodes_for_level(level));
			}
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		if !Self::is_structure_level(level) {
			return Layers::new();
		}
		// Caps and rects are the exterior deck, same band as the ring wall.
		// An Internal confine hid them until the camera was inside 5× radius,
		// so Medium (and the outer High shell) was a stone ring with clear
		// color where the inscribed-square caps should be.
		let mut out = Layers::from_free(
			self.floor_caps.iter().chain(self.floor_rects.iter()).cloned().collect(),
		);
		if Self::is_detail_level(level) {
			for room in &self.rooms {
				out.extend(room.floor_nodes_for_level(level));
			}
		}
		out
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		if !Self::is_structure_level(level) {
			return Layers::new();
		}
		Layers::from_free(vec![self
			.arc_spire
			.stairs
			.clone()
			.with_confines(self.storey_spire_capsule())])
	}
}

impl LodScene for WizardsTowerFloor {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::scene::LodSceneStatus {
		lod::scene::LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		self.emit_internal_features(&mut children, lod_ref);
		scene_children(children)
	}
}
