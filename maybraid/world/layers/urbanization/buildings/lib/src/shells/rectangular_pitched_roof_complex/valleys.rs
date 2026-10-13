//! Pitch-plane valleys and rail truncation at junctions.
//!
//! Perp strip-back shortens long-axis extents on stems / L-arms; facing eaves
//! land on the valley. Coaxial end-meets park eaves/walls on the higher
//! volume's end wall plane; after end-caps, the run ridge is extended to the
//! hip apex (or left at the gable wall).

use bevy_math::Vec3;

use super::geometry::{LongAxis, Plane, VolumeCandidate, EPS};
use super::topology::{CoaxialMeet, ConcaveCorner, JunctionSet};
use super::{EndCap, RidgeJunction};

/// Valley / step segment (magenta gizmo line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValleySegment {
	pub eave_point: Vec3,
	pub ridge_point: Vec3,
	pub vol_a: usize,
	pub vol_b: usize,
}

/// Build valleys and truncate candidate rails so neighboring pitches meet on them.
pub(super) fn apply_valleys(
	volumes: &mut [VolumeCandidate],
	junctions: &JunctionSet,
	junction: RidgeJunction,
) -> Vec<ValleySegment> {
	let mut valleys = Vec::new();
	for corner in &junctions.perp {
		let Some(valley) = build_perp_valley(volumes, corner, junction) else {
			continue;
		};
		truncate_for_perp(volumes, corner, &valley);
		valleys.push(valley);
	}
	for meet in &junctions.coaxial {
		let Some(valley) = build_coaxial_end_valley(volumes, meet) else {
			continue;
		};
		truncate_coaxial_run(volumes, meet);
		valleys.push(valley);
	}
	valleys
}

fn build_perp_valley(
	volumes: &[VolumeCandidate],
	corner: &ConcaveCorner,
	junction: RidgeJunction,
) -> Option<ValleySegment> {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	debug_assert_eq!(a.long_axis, LongAxis::X);
	debug_assert_eq!(b.long_axis, LongAxis::Z);

	let plane_a = a.pitch_plane(corner.side_a)?;
	let plane_b = b.pitch_plane(corner.side_b)?;
	let (origin, dir) = Plane::intersect(plane_a, plane_b)?;

	let (cx, cz) = corner.corner_xz;
	let y_eave = a.eave[corner.side_a].a.y.min(b.eave[corner.side_b].a.y);
	let eave_x = if corner.side_b == 1 { cx + b.side_overhang } else { cx - b.side_overhang };
	let eave_z = if corner.side_a == 1 { cz + a.side_overhang } else { cz - a.side_overhang };
	let eave_point = Vec3::new(eave_x, y_eave, eave_z);
	let eave_on_valley = closest_point_on_line(origin, dir, eave_point);
	let ridge_point = ridge_junction_point(a, b, junction);

	Some(ValleySegment {
		eave_point: eave_on_valley,
		ridge_point,
		vol_a: corner.vol_a,
		vol_b: corner.vol_b,
	})
}

fn ridge_junction_point(a: &VolumeCandidate, b: &VolumeCandidate, junction: RidgeJunction) -> Vec3 {
	let y_join = junction.resolve(a.ridge.a.y, b.ridge.a.y);
	Vec3::new(b.ridge.a.x, y_join, a.ridge.a.z)
}

fn closest_point_on_line(origin: Vec3, dir: Vec3, p: Vec3) -> Vec3 {
	let dir = dir.normalize_or_zero();
	origin + dir * (p - origin).dot(dir)
}

/// Gizmo on the cap's end-gable plane: run ridge height → cap ridge end.
fn build_coaxial_end_valley(
	volumes: &[VolumeCandidate],
	meet: &CoaxialMeet,
) -> Option<ValleySegment> {
	let run = &volumes[meet.vol_run];
	let cap = &volumes[meet.vol_cap];
	let end = meet.run_end;

	let (low, high) = match meet.long_axis {
		LongAxis::X => {
			let x = match end {
				1 => cap.plan_min().0,
				_ => cap.plan_max().0,
			};
			let z = run.ridge.a.z;
			let y_run = run.ridge.a.y;
			let y_cap = cap.ridge.a.y;
			(Vec3::new(x, y_run.min(y_cap), z), Vec3::new(x, y_run.max(y_cap), z))
		}
		LongAxis::Z => {
			let z = match end {
				1 => cap.plan_min().1,
				_ => cap.plan_max().1,
			};
			let x = run.ridge.a.x;
			let y_run = run.ridge.a.y;
			let y_cap = cap.ridge.a.y;
			(Vec3::new(x, y_run.min(y_cap), z), Vec3::new(x, y_run.max(y_cap), z))
		}
	};

	Some(ValleySegment {
		eave_point: low,
		ridge_point: high,
		vol_a: meet.vol_run,
		vol_b: meet.vol_cap,
	})
}

/// Outside (convex) plan corner opposite a concave L corner, if both volumes
/// actually share that corner (true L). Cross-arm pairs fail this and skip the
/// outside-hip snap — otherwise outer eaves get pulled into the ridge crossing.
fn outside_eave_corner(volumes: &[VolumeCandidate], corner: &ConcaveCorner) -> Option<Vec3> {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	let (end_a, end_b) = (corner.end_a?, corner.end_b?);
	let outer_a = 1 - corner.side_a;
	let outer_b = 1 - corner.side_b;
	let (ox, oz) = corner_massing_outside(volumes, corner);
	if !plan_touches(a, ox, oz) || !plan_touches(b, ox, oz) {
		return None;
	}
	let ea = a.eave[outer_a].end(end_a);
	let eb = b.eave[outer_b].end(end_b);
	let y = ea.y.min(eb.y);
	Some(Vec3::new(eb.x, y, ea.z))
}

fn plan_touches(vol: &VolumeCandidate, x: f32, z: f32) -> bool {
	let (min_x, min_z) = vol.plan_min();
	let (max_x, max_z) = vol.plan_max();
	x >= min_x - EPS && x <= max_x + EPS && z >= min_z - EPS && z <= max_z + EPS
}

fn truncate_for_perp(
	volumes: &mut [VolumeCandidate],
	corner: &ConcaveCorner,
	valley: &ValleySegment,
) {
	let ridge_join = valley.ridge_point;

	truncate_long_x(&mut volumes[corner.vol_a], corner.side_a, corner.end_a, valley, ridge_join);
	truncate_long_z(&mut volumes[corner.vol_b], corner.side_b, corner.end_b, valley, ridge_join);

	// Close the outside hip only for true L footprints (both boxes share the
	// convex corner). Decomposed cross arms skip this.
	if let Some(outer) = outside_eave_corner(volumes, corner) {
		if let (Some(end_a), Some(end_b)) = (corner.end_a, corner.end_b) {
			let outer_a = 1 - corner.side_a;
			let outer_b = 1 - corner.side_b;
			let ya = volumes[corner.vol_a].eave[outer_a].end(end_a).y;
			let yb = volumes[corner.vol_b].eave[outer_b].end(end_b).y;
			volumes[corner.vol_a].eave[outer_a].set_end(end_a, Vec3::new(outer.x, ya, outer.z));
			volumes[corner.vol_b].eave[outer_b].set_end(end_b, Vec3::new(outer.x, yb, outer.z));
			let wa = volumes[corner.vol_a].wall[outer_a].end(end_a);
			let wb = volumes[corner.vol_b].wall[outer_b].end(end_b);
			let (cx, cz) = corner_massing_outside(volumes, corner);
			volumes[corner.vol_a].wall[outer_a].set_end(end_a, Vec3::new(cx, wa.y, cz));
			volumes[corner.vol_b].wall[outer_b].set_end(end_b, Vec3::new(cx, wb.y, cz));
		}
	}
}

fn corner_massing_outside(volumes: &[VolumeCandidate], corner: &ConcaveCorner) -> (f32, f32) {
	let a = &volumes[corner.vol_a];
	let b = &volumes[corner.vol_b];
	let (amin_x, amin_z) = a.plan_min();
	let (amax_x, _) = a.plan_max();
	let (bmin_x, bmin_z) = b.plan_min();
	let (_, bmax_z) = b.plan_max();
	let x = if corner.side_b == 1 { amin_x.min(bmin_x) } else { amax_x.max(b.plan_max().0) };
	let z = if corner.side_a == 1 { amin_z.min(bmin_z) } else { a.plan_max().1.max(bmax_z) };
	(x, z)
}

fn truncate_long_x(
	vol: &mut VolumeCandidate,
	side: usize,
	end: Option<usize>,
	valley: &ValleySegment,
	ridge_end: Vec3,
) {
	let Some(end) = end else {
		return;
	};

	vol.ridge.set_end(end, ridge_end);

	// Facing eave lands on the valley (true valley edge).
	let ey = vol.eave[side].end(end).y;
	vol.eave[side].set_end(end, Vec3::new(valley.eave_point.x, ey, valley.eave_point.z));
	// Outer eave / both walls: parallel long-axis clip only.
	let outer = 1 - side;
	let mut eo = vol.eave[outer].end(end);
	eo.x = ridge_end.x;
	vol.eave[outer].set_end(end, eo);
	for i in 0..2 {
		let mut w = vol.wall[i].end(end);
		w.x = ridge_end.x;
		vol.wall[i].set_end(end, w);
	}
}

fn truncate_long_z(
	vol: &mut VolumeCandidate,
	side: usize,
	end: Option<usize>,
	valley: &ValleySegment,
	ridge_end: Vec3,
) {
	let Some(end) = end else {
		return;
	};

	vol.ridge.set_end(end, ridge_end);

	let ey = vol.eave[side].end(end).y;
	vol.eave[side].set_end(end, Vec3::new(valley.eave_point.x, ey, valley.eave_point.z));
	let outer = 1 - side;
	let mut eo = vol.eave[outer].end(end);
	eo.z = ridge_end.z;
	vol.eave[outer].set_end(end, eo);
	for i in 0..2 {
		let mut w = vol.wall[i].end(end);
		w.z = ridge_end.z;
		vol.wall[i].set_end(end, w);
	}
}

/// After end-caps: meet the run ridge on the cap hip's centerline edge
/// (apex → end-wall drop), not under the higher ridge tip. Eaves stay on the
/// end-wall plane. Gable keeps the ridge at that wall (barge is cap-only).
pub(super) fn finish_coaxial_ridge_meets(
	volumes: &mut [VolumeCandidate],
	meets: &[CoaxialMeet],
	end_cap: EndCap,
	valleys: &mut [ValleySegment],
) {
	if !matches!(end_cap, EndCap::Hip) {
		return;
	}
	for meet in meets {
		// Run max butts cap min (end 0); run min butts cap max (end 1).
		let cap_end = 1 - meet.run_end;
		let run_y = volumes[meet.vol_run].ridge.a.y;
		let apex = volumes[meet.vol_cap].ridge.end(cap_end);
		let Some(meet_pt) =
			hip_centerline_meet(&volumes[meet.vol_cap], cap_end, meet.long_axis, run_y)
		else {
			continue;
		};
		{
			let run = &mut volumes[meet.vol_run];
			let mut r = run.ridge.end(meet.run_end);
			match meet.long_axis {
				LongAxis::X => r.x = meet_pt.x,
				LongAxis::Z => r.z = meet_pt.z,
			}
			run.ridge.set_end(meet.run_end, r);
		}

		for v in valleys.iter_mut() {
			if v.vol_a == meet.vol_run && v.vol_b == meet.vol_cap {
				// Gizmo along the hip edge: run meet → cap apex.
				v.eave_point = meet_pt;
				v.ridge_point = apex;
			}
		}
	}
}

/// Intersection of the run ridge height with the hip edge from ridge apex down
/// to the end-wall point under the ridge (shared base of the two end hips).
fn hip_centerline_meet(
	cap: &VolumeCandidate,
	cap_end: usize,
	long_axis: LongAxis,
	run_ridge_y: f32,
) -> Option<Vec3> {
	let apex = cap.ridge.end(cap_end);
	let eave_end = cap.eave[0].end(cap_end);
	// Same construction as pitched-roof `hip_drop`: end-wall point under the ridge.
	let drop = match long_axis {
		LongAxis::X => Vec3::new(eave_end.x, eave_end.y, apex.z),
		LongAxis::Z => Vec3::new(apex.x, eave_end.y, eave_end.z),
	};
	let dy = drop.y - apex.y;
	if dy.abs() < EPS {
		return None;
	}
	let t = ((run_ridge_y - apex.y) / dy).clamp(0.0, 1.0);
	Some(apex.lerp(drop, t))
}

/// Strip the run's long end to the cap's end-wall (massing) plane; leave cap eaves alone.
fn truncate_coaxial_run(volumes: &mut [VolumeCandidate], meet: &CoaxialMeet) {
	let plane_long = match meet.long_axis {
		LongAxis::X => {
			if meet.run_end == 1 {
				volumes[meet.vol_cap].plan_min().0
			} else {
				volumes[meet.vol_cap].plan_max().0
			}
		}
		LongAxis::Z => {
			if meet.run_end == 1 {
				volumes[meet.vol_cap].plan_min().1
			} else {
				volumes[meet.vol_cap].plan_max().1
			}
		}
	};

	let run = &mut volumes[meet.vol_run];
	let end = meet.run_end;
	match meet.long_axis {
		LongAxis::X => {
			let mut r = run.ridge.end(end);
			r.x = plane_long;
			run.ridge.set_end(end, r);
			for i in 0..2 {
				let mut e = run.eave[i].end(end);
				e.x = plane_long;
				run.eave[i].set_end(end, e);
				let mut w = run.wall[i].end(end);
				w.x = plane_long;
				run.wall[i].set_end(end, w);
			}
		}
		LongAxis::Z => {
			let mut r = run.ridge.end(end);
			r.z = plane_long;
			run.ridge.set_end(end, r);
			for i in 0..2 {
				let mut e = run.eave[i].end(end);
				e.z = plane_long;
				run.eave[i].set_end(end, e);
				let mut w = run.wall[i].end(end);
				w.z = plane_long;
				run.wall[i].set_end(end, w);
			}
		}
	}
}
