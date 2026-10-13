//! Domain-agnostic furniture propose/try helpers.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::{aabb3_to_plan, NoiseConfig, PlanAxes};

use super::predicates::{against_wall, clear_of_keep_outs, in_host, long_face_on_wall};

/// Knobs for a free-standing rectangular solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeExtentKnobs {
	pub extent: Vec3,
	pub prefer_wall: bool,
	pub wall_eps: f32,
	pub attempts: u32,
}

/// Knobs for a wall-long storage / counter run (`extent` = long, height, short).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallLongKnobs {
	pub extent: Vec3,
	pub wall_eps: f32,
	pub attempts: u32,
}

/// Sample a free extent inside `host` that clears keep-outs.
///
/// When [`FreeExtentKnobs::prefer_wall`] is set, wall-flush candidates are tried
/// first (same wall walk as [`try_wall_long`]), then free samples as fallback.
pub fn try_free_extent(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	salt: u32,
	knobs: FreeExtentKnobs,
) -> Option<Aabb3d> {
	let extent = knobs.extent;
	let size = host3.max - host3.min;
	if extent.x > size.x + 1e-3 || extent.z > size.z + 1e-3 {
		return None;
	}
	if knobs.prefer_wall {
		if let Some(a) = try_extent_on_walls(
			host3,
			host,
			clearances,
			cfg,
			salt,
			extent,
			knobs.wall_eps,
			knobs.attempts,
		) {
			return Some(a);
		}
	}
	let max_u = (size.x - extent.x).max(0.0);
	let max_v = (size.z - extent.z).max(0.0);
	for attempt in 0..knobs.attempts {
		let u = cfg.sample_unit_4d(salt as f32, attempt as f32, 0.0, 20.0);
		let v = cfg.sample_unit_4d(salt as f32, attempt as f32, 0.0, 21.0);
		let min = Vec3::new(host3.min.x + u * max_u, host3.min.y, host3.min.z + v * max_v);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
		if !in_host(host, plan, 1e-3) || !clear_of_keep_outs(plan, clearances) {
			continue;
		}
		if knobs.prefer_wall && !against_wall(host, plan, knobs.wall_eps) {
			continue;
		}
		return Some(candidate);
	}
	// Last resort when prefer_wall blocked every free sample: place without wall.
	if knobs.prefer_wall {
		let mut relaxed = knobs;
		relaxed.prefer_wall = false;
		return try_free_extent(host3, host, clearances, cfg, salt.wrapping_add(50), relaxed);
	}
	None
}

/// Place a box with its long face on a host wall.
pub fn try_wall_long(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	salt: u32,
	knobs: WallLongKnobs,
) -> Option<Aabb3d> {
	let long = knobs.extent.x;
	let short = knobs.extent.z;
	let height = knobs.extent.y;
	let size = host3.max - host3.min;
	if long.min(short) > size.x.min(size.z) + 1e-3 {
		return None;
	}
	let start = (cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 40.0) * 4.0).floor() as u32 % 4;
	for k in 0..4u32 {
		let wall = (start + k) % 4;
		let e = match wall {
			0 | 1 => Vec3::new(long, height, short),
			_ => Vec3::new(short, height, long),
		};
		if e.x > size.x + 1e-3 || e.z > size.z + 1e-3 {
			continue;
		}
		let max_u = (size.x - e.x).max(0.0);
		let max_v = (size.z - e.z).max(0.0);
		for attempt in 0..knobs.attempts {
			let t = cfg.sample_unit_4d(salt as f32, attempt as f32, wall as f32, 41.0);
			let min = match wall {
				0 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.min.z),
				1 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.max.z - e.z),
				2 => Vec3::new(host3.min.x, host3.min.y, host3.min.z + t * max_v),
				_ => Vec3::new(host3.max.x - e.x, host3.min.y, host3.min.z + t * max_v),
			};
			let candidate = Aabb3d::from_min_max(min, min + e);
			let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
			if in_host(host, plan, 1e-3)
				&& clear_of_keep_outs(plan, clearances)
				&& long_face_on_wall(host, plan, knobs.wall_eps)
			{
				return Some(candidate);
			}
		}
	}
	None
}

/// Place `extent` flush on a host wall (tries both in-plane orientations).
fn try_extent_on_walls(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	salt: u32,
	extent: Vec3,
	wall_eps: f32,
	attempts: u32,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	let start = (cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 22.0) * 4.0).floor() as u32 % 4;
	for k in 0..4u32 {
		let wall = (start + k) % 4;
		for &swap in &[false, true] {
			let e = if swap { Vec3::new(extent.z, extent.y, extent.x) } else { extent };
			if e.x > size.x + 1e-3 || e.z > size.z + 1e-3 {
				continue;
			}
			let max_u = (size.x - e.x).max(0.0);
			let max_v = (size.z - e.z).max(0.0);
			for attempt in 0..attempts {
				let t = cfg.sample_unit_4d(
					salt as f32,
					attempt as f32,
					wall as f32 + (swap as u32 as f32) * 0.5,
					23.0,
				);
				let min = match wall {
					0 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.min.z),
					1 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.max.z - e.z),
					2 => Vec3::new(host3.min.x, host3.min.y, host3.min.z + t * max_v),
					_ => Vec3::new(host3.max.x - e.x, host3.min.y, host3.min.z + t * max_v),
				};
				let candidate = Aabb3d::from_min_max(min, min + e);
				let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
				if in_host(host, plan, 1e-3)
					&& clear_of_keep_outs(plan, clearances)
					&& against_wall(host, plan, wall_eps)
				{
					return Some(candidate);
				}
			}
		}
	}
	None
}

/// Build a plan AABB from origin + size (XZ as Aabb2d x/y).
pub fn plan_rect(ox: f32, oz: f32, sx: f32, sz: f32) -> Aabb2d {
	Aabb2d { min: Vec2::new(ox, oz), max: Vec2::new(ox + sx, oz + sz) }
}
