//! Multi-run composition helpers (L-corner, peninsula stub).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{aabb3_to_plan, NoiseConfig, PlanAxes};

use super::pack::WALL_EPS;
use super::predicates::{clear_of_keep_outs, in_host, long_face_on_wall};

/// Which host wall a plan AABB is flush against (0=−Z, 1=+Z, 2=−X, 3=+X).
pub fn wall_of(plan: Aabb2d, host: Aabb2d) -> Option<u32> {
	if (plan.min.y - host.min.y).abs() <= WALL_EPS {
		Some(0)
	} else if (plan.max.y - host.max.y).abs() <= WALL_EPS {
		Some(1)
	} else if (plan.min.x - host.min.x).abs() <= WALL_EPS {
		Some(2)
	} else if (plan.max.x - host.max.x).abs() <= WALL_EPS {
		Some(3)
	} else {
		None
	}
}

pub fn plans_touch(a: Aabb2d, b: Aabb2d) -> bool {
	const EPS: f32 = 0.05;
	let gap_x = (a.min.x - b.max.x).max(b.min.x - a.max.x);
	let gap_y = (a.min.y - b.max.y).max(b.min.y - a.max.y);
	gap_x <= EPS && gap_y <= EPS
}

/// Corner-seeded L: both runs start at the same host corner so they meet.
pub fn try_corner_l(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	along_a: f32,
	along_b: f32,
	depth: f32,
	height: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let start = (cfg.sample_unit_4d(0.0, 0.0, 0.0, 45.0) * 4.0).floor() as u32 % 4;
	let y0 = host3.min.y;
	for k in 0..4u32 {
		let corner = (start + k) % 4;
		let (a, b) = corner_l_runs(host, corner, along_a, along_b, depth, height, y0)?;
		let pa = aabb3_to_plan(&a, PlanAxes::XZ);
		let pb = aabb3_to_plan(&b, PlanAxes::XZ);
		if !in_host(host, pa, 1e-3)
			|| !in_host(host, pb, 1e-3)
			|| !long_face_on_wall(host, pa, WALL_EPS)
			|| !long_face_on_wall(host, pb, WALL_EPS)
			|| !clear_of_keep_outs(pa, clearances)
			|| !clear_of_keep_outs(pb, clearances)
		{
			continue;
		}
		if !plans_touch(pa, pb) {
			continue;
		}
		return Some((a, b));
	}
	None
}

/// `corner`: 0=SW (−X,−Z), 1=SE (+X,−Z), 2=NW (−X,+Z), 3=NE (+X,+Z).
pub fn corner_l_runs(
	host: Aabb2d,
	corner: u32,
	along_x: f32,
	along_z: f32,
	depth: f32,
	height: f32,
	y0: f32,
) -> Option<(Aabb3d, Aabb3d)> {
	let span_x = (host.max.x - host.min.x).max(0.0);
	let span_z = (host.max.y - host.min.y).max(0.0);
	let ax = along_x.min(span_x);
	let az = along_z.min(span_z);
	if ax < depth + 0.2 || az < depth + 0.2 {
		return None;
	}
	let (run_x_min, run_z_min) = match corner {
		0 => {
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.min.y),
				Vec3::new(host.min.x + ax, y0 + height, host.min.y + depth),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.min.y),
				Vec3::new(host.min.x + depth, y0 + height, host.min.y + az),
			);
			(run_x, run_z)
		}
		1 => {
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.max.x - ax, y0, host.min.y),
				Vec3::new(host.max.x, y0 + height, host.min.y + depth),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.max.x - depth, y0, host.min.y),
				Vec3::new(host.max.x, y0 + height, host.min.y + az),
			);
			(run_x, run_z)
		}
		2 => {
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.max.y - depth),
				Vec3::new(host.min.x + ax, y0 + height, host.max.y),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.min.x, y0, host.max.y - az),
				Vec3::new(host.min.x + depth, y0 + height, host.max.y),
			);
			(run_x, run_z)
		}
		_ => {
			let run_x = Aabb3d::from_min_max(
				Vec3::new(host.max.x - ax, y0, host.max.y - depth),
				Vec3::new(host.max.x, y0 + height, host.max.y),
			);
			let run_z = Aabb3d::from_min_max(
				Vec3::new(host.max.x - depth, y0, host.max.y - az),
				Vec3::new(host.max.x, y0 + height, host.max.y),
			);
			(run_x, run_z)
		}
	};
	Some((run_x_min, run_z_min))
}

/// Peninsula stub from one end of a wall run into the room (perpendicular).
pub fn try_peninsula_from_run(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	primary: &Aabb3d,
	along: f32,
	depth: f32,
	height: f32,
	cfg: &NoiseConfig,
	salt: u32,
) -> Option<Aabb3d> {
	let p = aabb3_to_plan(primary, PlanAxes::XZ);
	let wall = wall_of(p, host)?;
	let use_hi = cfg.sample_unit_4d(salt as f32, 0.0, 0.0, 44.0) >= 0.5;
	let y0 = host3.min.y;
	let candidate = match wall {
		0 => {
			let x0 = if use_hi { p.max.x - depth } else { p.min.x };
			let min = Vec3::new(x0, y0, p.max.y);
			Aabb3d::from_min_max(min, min + Vec3::new(depth, height, along))
		}
		1 => {
			let x0 = if use_hi { p.max.x - depth } else { p.min.x };
			let min = Vec3::new(x0, y0, p.min.y - along);
			Aabb3d::from_min_max(min, min + Vec3::new(depth, height, along))
		}
		2 => {
			let z0 = if use_hi { p.max.y - depth } else { p.min.y };
			let min = Vec3::new(p.max.x, y0, z0);
			Aabb3d::from_min_max(min, min + Vec3::new(along, height, depth))
		}
		_ => {
			let z0 = if use_hi { p.max.y - depth } else { p.min.y };
			let min = Vec3::new(p.min.x - along, y0, z0);
			Aabb3d::from_min_max(min, min + Vec3::new(along, height, depth))
		}
	};
	let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
	if !in_host(host, plan, 1e-3) || !plans_touch(plan, p) {
		return None;
	}
	let others: Vec<Aabb2d> =
		clearances.iter().copied().filter(|c| !aabb2_near_eq(*c, p, 1e-3)).collect();
	if !clear_of_keep_outs(plan, &others) {
		return None;
	}
	let w = plan.max.x - plan.min.x;
	let d = plan.max.y - plan.min.y;
	let long_on_wall = if w >= d {
		(plan.min.y - host.min.y).abs() <= WALL_EPS || (plan.max.y - host.max.y).abs() <= WALL_EPS
	} else {
		(plan.min.x - host.min.x).abs() <= WALL_EPS || (plan.max.x - host.max.x).abs() <= WALL_EPS
	};
	if long_on_wall {
		return None;
	}
	Some(candidate)
}

fn aabb2_near_eq(a: Aabb2d, b: Aabb2d, eps: f32) -> bool {
	(a.min.x - b.min.x).abs() < eps
		&& (a.min.y - b.min.y).abs() < eps
		&& (a.max.x - b.max.x).abs() < eps
		&& (a.max.y - b.max.y).abs() < eps
}
