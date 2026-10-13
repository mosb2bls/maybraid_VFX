//! Shared pre-pocket AABB helpers for dual-band Watershed stacks.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use procedural_common::Bounds2;
use terrain_watersheds::PrePocket;

/// Build an AABB for a pocket tile (XZ from [`PrePocket`], Y from parent cell).
pub fn pocket_aabb(pre: &PrePocket, px: u32, pz: u32, vy_min: f32, vy_max: f32) -> Aabb3d {
	aabb_from_bounds2(pre.pocket_bounds(px, pz), vy_min, vy_max)
}

pub fn aabb_from_bounds2(b: Bounds2, vy_min: f32, vy_max: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(b.min.x, vy_min, b.min.y), Vec3::new(b.max.x, vy_max, b.max.y))
}
