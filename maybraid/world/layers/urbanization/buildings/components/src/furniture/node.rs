//! Furniture IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::furniture::abutment::FurnitureAbutment;
use crate::furniture::geometry::FurnitureGeometry;
use crate::furniture::style::FurnitureStyle;
use crate::furniture::wireframe::FurnitureWireframeAssets;
use crate::lod_band::placement_bounds;
use crate::placed::Placement;
use crate::scene_children::{pose, wireframe_box_with_handles};

/// Authoring IR for a furniture / fixture **slot**.
///
/// World pose is the `DevelopmentHost` transform composed with [`Self::placement`]
/// (same as other building kits). [`Self::abutment`] and `placement.yaw` stay in
/// that building-local frame so a later assembly pass can face the kit without
/// re-reading the floor plan.
///
/// Until kit GLBs exist, [`FurnitureStyle::Placeholder`] draws a wireframe cube.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct FurnitureNode {
	pub style: FurnitureStyle,
	pub geometry: FurnitureGeometry,
	pub placement: Placement,
	/// Host / partition face the packed box flushes to, when the packer said so.
	pub abutment: Option<FurnitureAbutment>,
	/// Stable palette key for a later finish pass (not a development-kind walk).
	pub finish_seed: u64,
}

impl FurnitureNode {
	pub fn new(style: FurnitureStyle, geometry: FurnitureGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, abutment: None, finish_seed: 0 }
	}

	pub fn placeholder(geometry: FurnitureGeometry, placement: Placement) -> Self {
		Self::new(FurnitureStyle::Placeholder, geometry, placement)
	}

	pub fn bed(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Bed, placement)
	}

	pub fn wardrobe(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Wardrobe, placement)
	}

	pub fn dresser(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Dresser, placement)
	}

	pub fn nightstand(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Nightstand, placement)
	}

	pub fn bedroom_furniture(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::BedroomFurniture, placement)
	}

	pub fn toilet(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Toilet, placement)
	}

	pub fn chair(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Chair, placement)
	}

	pub fn table(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Table, placement)
	}

	pub fn chest(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Chest, placement)
	}

	pub fn counter(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Counter, placement)
	}

	pub fn shelf(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Shelf, placement)
	}

	pub fn range(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Range, placement)
	}

	pub fn basin(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Basin, placement)
	}

	pub fn faucet(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Faucet, placement)
	}

	pub fn food_display(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::FoodDisplay, placement)
	}

	pub fn fridge(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Fridge, placement)
	}

	pub fn fruit(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Fruit, placement)
	}

	pub fn bread(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Bread, placement)
	}

	pub fn cookware(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Cookware, placement)
	}

	pub fn partition(placement: Placement) -> Self {
		Self::placeholder(FurnitureGeometry::Partition, placement)
	}

	pub fn with_abutment(mut self, abutment: FurnitureAbutment) -> Self {
		self.abutment = Some(abutment);
		self.placement.yaw = abutment.facing_yaw();
		self
	}

	pub fn with_finish_seed(mut self, finish_seed: u64) -> Self {
		self.finish_seed = finish_seed;
		self
	}

	/// Stamp pose, abutment, and finish seed from the packed AABB versus host.
	///
	/// Flush walls come from the packer's committed box, not a second room walk.
	/// Free boxes use the shorter-axis / nearer-wall yaw convention. Scale is
	/// kit-local so a ±90° yaw still fills `slot` (not a rotated world AABB).
	pub fn stamp_host_slot(&mut self, slot: &Aabb3d, host: &Aabb3d) {
		self.finish_seed = finish_seed_for(self.geometry, slot);
		self.placement.translation = bevy::math::Vec3::from((slot.min + slot.max) * 0.5);
		if let Some(abutment) = FurnitureAbutment::from_flush(slot, host) {
			self.abutment = Some(abutment);
			self.placement.yaw = abutment.facing_yaw();
			self.placement.scale = abutment.local_scale(slot);
		} else {
			self.abutment = None;
			self.placement.yaw = FurnitureAbutment::free_facing_yaw(slot, host);
			self.placement.scale =
				crate::furniture::abutment::local_scale_for_yaw(slot, self.placement.yaw);
		}
	}
}

/// Mix geometry + slot center into a stable finish key.
pub fn finish_seed_for(geometry: FurnitureGeometry, slot: &Aabb3d) -> u64 {
	let center = (slot.min + slot.max) * 0.5;
	let mut value = geometry.finish_salt()
		^ (center.x.to_bits() as u64).rotate_left(7)
		^ (center.y.to_bits() as u64).rotate_left(17)
		^ (center.z.to_bits() as u64).rotate_left(29);
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

impl LodScene for FurnitureNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		match self.style {
			FurnitureStyle::Placeholder => {
				let mesh = FurnitureWireframeAssets::unit_cube();
				let material = FurnitureWireframeAssets::material_for(self.geometry);
				wireframe_box_with_handles(mesh, material, pose(self.placement))
			}
		}
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
