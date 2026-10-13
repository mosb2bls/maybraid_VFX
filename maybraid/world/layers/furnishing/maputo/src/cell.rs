//! 50 m furniture cells. These are the furniture [`lod::LodSceneHost`]s.

use bevy::math::bounding::Aabb3d;
use bevy::math::{EulerRot, Vec3};
use bevy::prelude::Transform;
use building_components::{FurnitureNode, Placement};
use lod::gen::Id;

/// Square furniture-cell edge length (metres).
pub const FURNITURE_CELL_SIZE: f32 = 50.0;
/// High-LOD / present neighborhood half-span: current cell plus neighbors.
pub const FURNITURE_PRESENT_RADIUS: f32 = 125.0;
/// Generate a ring past present so neighbors exist before the camera crosses.
pub const FURNITURE_GENERATE_RADIUS: f32 = 250.0;
/// Half-height of generate / present / refresh AABBs. Durham sits near \(y \approx -140\);
/// a sea-level `0..1` slab misses both developments and Gimme host queries.
pub const FURNITURE_REGION_Y_HALF: f32 = 2048.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FurnitureCellExtent {
	pub min: Vec3,
	pub max: Vec3,
}

impl FurnitureCellExtent {
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let half = FURNITURE_CELL_SIZE * 0.5;
		Self {
			min: Vec3::new(
				ix as f32 * FURNITURE_CELL_SIZE - half,
				0.0,
				iz as f32 * FURNITURE_CELL_SIZE - half,
			),
			max: Vec3::new(
				ix as f32 * FURNITURE_CELL_SIZE + half,
				1.0,
				iz as f32 * FURNITURE_CELL_SIZE + half,
			),
		}
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		let width = bounds.max.x - bounds.min.x;
		let depth = bounds.max.z - bounds.min.z;
		if (width - FURNITURE_CELL_SIZE).abs() > 1e-3 || (depth - FURNITURE_CELL_SIZE).abs() > 1e-3
		{
			return None;
		}
		Some(Self { min: bounds.min.into(), max: bounds.max.into() })
	}

	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let min = Self::cell_index_containing(Vec3::new(region.min.x, 0.0, region.min.z));
		let max = Self::cell_index_containing(Vec3::new(
			(region.max.x - 1e-3).max(region.min.x),
			0.0,
			(region.max.z - 1e-3).max(region.min.z),
		));
		(min.0.min(max.0)..=min.0.max(max.0))
			.flat_map(|ix| {
				(min.1.min(max.1)..=min.1.max(max.1)).map(move |iz| Self::from_cell_index(ix, iz))
			})
			.collect()
	}

	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let half = FURNITURE_CELL_SIZE * 0.5;
		(
			((position.x + half) / FURNITURE_CELL_SIZE).floor() as i32,
			((position.z + half) / FURNITURE_CELL_SIZE).floor() as i32,
		)
	}

	pub fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn index(self) -> (i32, i32) {
		Self::cell_index_containing(self.center())
	}

	pub fn contains_xz(self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
	}
}

/// Host transform as a [`Placement`] so slot IR can [`Placement::compose_child`].
pub fn placement_from_transform(transform: Transform) -> Placement {
	let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
	Placement { translation: transform.translation, yaw, pitch, roll, scale: transform.scale }
}

/// Bake a host-local slot into world space.
pub fn world_slot(host: Transform, mut node: FurnitureNode) -> FurnitureNode {
	node.placement = placement_from_transform(host).compose_child(node.placement);
	node
}

pub fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
	let r = radius.max(0.0);
	Aabb3d::from_min_max(
		Vec3::new(center.x - r, center.y - FURNITURE_REGION_Y_HALF, center.z - r),
		Vec3::new(center.x + r, center.y + FURNITURE_REGION_Y_HALF, center.z + r),
	)
}

/// Keep / present / Gimme rings are XZ. Y is not a furniture live axis.
pub fn intersects_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

#[cfg(test)]
mod tests {
	use bevy::math::bounding::IntersectsVolume;

	use super::*;

	#[test]
	fn origin_bins_to_cell_zero() {
		assert_eq!(FurnitureCellExtent::cell_index_containing(Vec3::ZERO), (0, 0));
		let cell = FurnitureCellExtent::from_cell_index(0, 0);
		assert!(cell.contains_xz(Vec3::ZERO));
		assert!(!cell.contains_xz(Vec3::new(25.0, 0.0, 0.0)));
	}

	#[test]
	fn neighbor_cell_is_fifty_metres() {
		let cell = FurnitureCellExtent::from_cell_index(1, 0);
		assert!((cell.center().x - 50.0).abs() < 1e-4);
		assert_eq!(FurnitureCellExtent::cell_index_containing(Vec3::new(30.0, 0.0, 0.0)), (1, 0));
	}

	#[test]
	fn world_slot_composes_host_yaw() {
		let host = Transform::from_xyz(10.0, 2.0, -4.0);
		let node = FurnitureNode::chest(Placement::new(Vec3::new(1.0, 0.0, 0.0), 0.0));
		let world = world_slot(host, node);
		assert!((world.placement.translation.x - 11.0).abs() < 1e-4);
		assert!((world.placement.translation.z + 4.0).abs() < 1e-4);
	}

	#[test]
	fn keep_aabb_follows_camera_height() {
		let region = xz_radius_aabb(Vec3::new(900.0, -141.0, 1800.0), 125.0);
		assert!(region.min.y < -140.0);
		assert!(region.max.y > -140.0);
		assert!((region.max.x - region.min.x - 250.0).abs() < 1e-3);
	}

	#[test]
	fn xz_overlap_ignores_height() {
		let terrain =
			Aabb3d::from_min_max(Vec3::new(0.0, -200.0, 0.0), Vec3::new(50.0, -100.0, 50.0));
		let sea = Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 10.0), Vec3::new(20.0, 1.0, 20.0));
		assert!(intersects_xz(terrain, sea));
		assert!(!terrain.intersects(&sea));
	}
}
