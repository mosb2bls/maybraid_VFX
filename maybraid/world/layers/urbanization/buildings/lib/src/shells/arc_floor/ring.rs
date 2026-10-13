//! Ring locus helpers shared by wall / opening resolution.
//!
//! Directions follow [`building_components::arc_ring_dir`] (kit on \(+X\)).

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::arc_kit::arc_ring_dir_deg;

use super::ArcFloorParams;

/// Kit segment size (degrees).
pub(super) const SEG_DEG: f32 = 15.0;
pub(super) const SECTORS: u32 = 24; // 360 / 15
pub(super) const EPS: f32 = 1e-4;

impl ArcFloorParams {
	/// Outward unit direction in XZ at normalized sweep parameter \(t\).
	pub(super) fn ring_dir_at(&self, t: f32) -> Vec2 {
		ring_dir_at(t)
	}

	/// World point on the ring exterior at yaw `deg` (floor elevation).
	pub(super) fn ring_point_deg(&self, deg: f32) -> Vec3 {
		let dir = arc_ring_dir_deg(deg);
		Vec3::new(
			self.center_xz.x + dir.x * self.radius,
			self.center_xz.y,
			self.center_xz.z + dir.y * self.radius,
		)
	}

	/// AABB approximating the kit sector at yaw `sector·15°`.
	pub(super) fn sector_aabb(&self, sector: u32) -> Aabb3d {
		let start = sector as f32 * SEG_DEG;
		let r_in = self.radius * 0.85;
		let r_out = self.radius * 1.05;
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		for step in 0..=2 {
			let deg = start - SEG_DEG * (step as f32 / 2.0);
			let d = arc_ring_dir_deg(deg);
			for r in [r_in, r_out] {
				for y in [y0, y1] {
					let p = Vec3::new(self.center_xz.x + d.x * r, y, self.center_xz.z + d.y * r);
					min = min.min(p);
					max = max.max(p);
				}
			}
		}
		Aabb3d::from_min_max(min, max)
	}
}

pub(super) fn ring_dir_at(t: f32) -> Vec2 {
	arc_ring_dir_deg(norm_t(t) * 360.0)
}

pub(super) fn norm_t(t: f32) -> f32 {
	let mut t = t % 1.0;
	if t < 0.0 {
		t += 1.0;
	}
	t
}

pub(super) fn aabb3d_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - EPS
		&& a.max.x > b.min.x + EPS
		&& a.min.y < b.max.y - EPS
		&& a.max.y > b.min.y + EPS
		&& a.min.z < b.max.z - EPS
		&& a.max.z > b.min.z + EPS
}
