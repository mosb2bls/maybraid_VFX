//! Packed-box helpers shared by usage expanders.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use building_components::{FurnitureAbutment, FurnitureNode, Placement};

/// Packed counter AABBs are often storey-tall; the kit is one world unit high.
pub const COUNTER_SLOT_HEIGHT: f32 = 1.0;

/// Sit `height` on the floor of `aabb`, keeping the XZ box.
pub fn floor_height_aabb(aabb: &Aabb3d, height: f32) -> Aabb3d {
	let y0 = aabb.min.y;
	Aabb3d::from_min_max(
		Vec3::new(aabb.min.x, y0, aabb.min.z),
		Vec3::new(aabb.max.x, y0 + height.max(1e-4), aabb.max.z),
	)
}

/// Run direction of a wall-hugging band: along the face, depth toward the room.
pub fn along_is_x(region: &Aabb3d, abutment: Option<FurnitureAbutment>) -> bool {
	match abutment {
		Some(FurnitureAbutment::NegX | FurnitureAbutment::PosX) => false,
		Some(FurnitureAbutment::NegZ | FurnitureAbutment::PosZ) => true,
		None => (region.max.x - region.min.x) + 1e-4 >= (region.max.z - region.min.z),
	}
}

pub fn along_span(region: &Aabb3d, along_x: bool) -> f32 {
	if along_x {
		region.max.x - region.min.x
	} else {
		region.max.z - region.min.z
	}
}

pub fn depth_span(region: &Aabb3d, along_x: bool) -> f32 {
	if along_x {
		region.max.z - region.min.z
	} else {
		region.max.x - region.min.x
	}
}

/// Slice `t0..t1` metres along the run, keeping the full depth and height.
pub fn slice_along(region: &Aabb3d, along_x: bool, t0: f32, t1: f32) -> Aabb3d {
	let t0 = t0.max(0.0);
	let t1 = t1.max(t0 + 1e-4);
	if along_x {
		Aabb3d::from_min_max(
			Vec3::new(region.min.x + t0, region.min.y, region.min.z),
			Vec3::new(region.min.x + t1, region.max.y, region.max.z),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(region.min.x, region.min.y, region.min.z + t0),
			Vec3::new(region.max.x, region.max.y, region.min.z + t1),
		)
	}
}

/// Wall-flush strip of `depth` on `side`, inset `end_pad` on the run.
pub fn wall_strip(region: &Aabb3d, side: FurnitureAbutment, depth: f32, end_pad: f32) -> Aabb3d {
	let depth = depth.max(1e-4);
	let pad = end_pad.max(0.0);
	match side {
		FurnitureAbutment::NegX => Aabb3d::from_min_max(
			Vec3::new(region.min.x, region.min.y, region.min.z + pad),
			Vec3::new(region.min.x + depth, region.max.y, region.max.z - pad),
		),
		FurnitureAbutment::PosX => Aabb3d::from_min_max(
			Vec3::new(region.max.x - depth, region.min.y, region.min.z + pad),
			Vec3::new(region.max.x, region.max.y, region.max.z - pad),
		),
		FurnitureAbutment::NegZ => Aabb3d::from_min_max(
			Vec3::new(region.min.x + pad, region.min.y, region.min.z),
			Vec3::new(region.max.x - pad, region.max.y, region.min.z + depth),
		),
		FurnitureAbutment::PosZ => Aabb3d::from_min_max(
			Vec3::new(region.min.x + pad, region.min.y, region.max.z - depth),
			Vec3::new(region.max.x - pad, region.max.y, region.max.z),
		),
	}
}

/// Longest host face, used when a kitchen remainder has no recorded flush.
pub fn longest_wall(region: &Aabb3d) -> FurnitureAbutment {
	let sx = region.max.x - region.min.x;
	let sz = region.max.z - region.min.z;
	if sx + 1e-4 >= sz {
		FurnitureAbutment::NegZ
	} else {
		FurnitureAbutment::NegX
	}
}

pub fn sit_on_aabb(slab: &Aabb3d, size: Vec3) -> Aabb3d {
	let c = (slab.min + slab.max) * 0.5;
	let pad = 0.03;
	let hx = (size.x * 0.5).min(((slab.max.x - slab.min.x) * 0.5 - pad).max(0.06));
	let hz = (size.z * 0.5).min(((slab.max.z - slab.min.z) * 0.5 - pad).max(0.06));
	let y0 = slab.max.y;
	Aabb3d::from_min_max(
		Vec3::new(c.x - hx, y0, c.z - hz),
		Vec3::new(c.x + hx, y0 + size.y.max(1e-4), c.z + hz),
	)
}

pub fn stamp_slot(
	mut node: FurnitureNode,
	slot: &Aabb3d,
	host: &Aabb3d,
	abutment: Option<FurnitureAbutment>,
) -> FurnitureNode {
	node.stamp_host_slot(slot, host);
	if let Some(side) = abutment {
		node = node.with_abutment(side);
		node.placement.scale = side.local_scale(slot);
	}
	node
}

pub fn stamp_make(
	make: fn(Placement) -> FurnitureNode,
	slot: &Aabb3d,
	host: &Aabb3d,
	abutment: Option<FurnitureAbutment>,
) -> FurnitureNode {
	stamp_slot(make(Placement::IDENTITY), slot, host, abutment)
}

/// Same as [`stamp_make`], then add `extra_yaw` (sit-on spin, chair toward a table).
pub fn stamp_make_yaw(
	make: fn(Placement) -> FurnitureNode,
	slot: &Aabb3d,
	host: &Aabb3d,
	abutment: Option<FurnitureAbutment>,
	extra_yaw: f32,
) -> FurnitureNode {
	let mut node = stamp_make(make, slot, host, abutment);
	node.placement.yaw += extra_yaw;
	node
}

/// Drop the wall strip of `depth` so the leftover is the room interior.
pub fn cut_from_wall(region: &Aabb3d, side: FurnitureAbutment, depth: f32) -> Aabb3d {
	let depth = depth.max(0.0);
	match side {
		FurnitureAbutment::NegX => Aabb3d::from_min_max(
			Vec3::new((region.min.x + depth).min(region.max.x - 1e-4), region.min.y, region.min.z),
			region.max,
		),
		FurnitureAbutment::PosX => Aabb3d::from_min_max(
			region.min,
			Vec3::new((region.max.x - depth).max(region.min.x + 1e-4), region.max.y, region.max.z),
		),
		FurnitureAbutment::NegZ => Aabb3d::from_min_max(
			Vec3::new(region.min.x, region.min.y, (region.min.z + depth).min(region.max.z - 1e-4)),
			region.max,
		),
		FurnitureAbutment::PosZ => Aabb3d::from_min_max(
			region.min,
			Vec3::new(region.max.x, region.max.y, (region.max.z - depth).max(region.min.z + 1e-4)),
		),
	}
}

/// Quarter-turn in `[0, 2π)` from a finish seed.
pub fn yaw_turns(seed: u64, salt: u64) -> f32 {
	std::f32::consts::FRAC_PI_2 * ((unit(seed, salt) * 4.0).floor())
}

/// Scale down a 2D grid so `n_x * n_z <= max`, preserving aspect ratio.
pub fn clamp_grid(n_x: usize, n_z: usize, max: usize) -> (usize, usize) {
	if n_x * n_z <= max {
		return (n_x, n_z);
	}
	let scale = (max as f32 / (n_x * n_z) as f32).sqrt();
	let nx = ((n_x as f32 * scale).floor() as usize).max(1);
	let nz = (max / nx).max(1);
	(nx, nz)
}

/// Unit in `[0, 1)` from a finish seed and salt.
pub fn unit(seed: u64, salt: u64) -> f32 {
	let mut x = seed ^ salt.wrapping_mul(0x9e37_79b9_7f4a_7c15);
	x ^= x >> 30;
	x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	x ^= x >> 27;
	x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
	x ^= x >> 31;
	(x as f32) * (1.0 / u64::MAX as f32)
}

#[cfg(test)]
mod tests {
	use super::clamp_grid;

	#[test]
	fn clamp_grid_within_cap_is_unchanged() {
		assert_eq!(clamp_grid(3, 4, 24), (3, 4));
	}

	#[test]
	fn clamp_grid_scales_down_preserving_product_bound() {
		let (n_x, n_z) = clamp_grid(8, 8, 24);
		assert!(n_x * n_z <= 24);
		assert!(n_x >= 1);
		assert!(n_z >= 1);
	}
}
