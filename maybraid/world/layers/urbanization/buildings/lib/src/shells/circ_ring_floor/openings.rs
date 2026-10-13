//! Ring opening plan helpers and construct-time mapped contact geometry.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::arc_kit::arc_ring_dir_deg;
use building_components::partitions::PartitionStyle;
use std::f32::consts::TAU;

use crate::arcs::ClippedArcSweep;
use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId,
	OpeningLabel, Openings,
};
use crate::shells::arc_floor::ArcFloor;

use super::{CircRingFloor, CircRingFloorParams};

const EPS: f32 = 1e-4;

/// Which circular wall an authored opening targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircRingWall {
	Outer,
	Inner,
}

impl CircRingFloor {
	/// Authoring helper: thin passage/aperture AABB on a ring at normalized \(t\).
	pub fn plan_opening_at_t(
		id: impl Into<OpeningId>,
		label: OpeningLabel,
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		t: f32,
	) -> (OpeningId, Opening) {
		ArcFloor::plan_opening_at_t(id, label, center_xz, radius, storey_height, t)
	}
}

impl CircRingFloorParams {
	pub(super) fn resolve_walls(
		&self,
	) -> (ClippedArcSweep, ClippedArcSweep, Openings, MappedOpenings) {
		let (outer_intervals, outer_openings, outer_mapped) =
			collect_ring_clips(self, CircRingWall::Outer);
		let (inner_intervals, inner_openings, inner_mapped) =
			collect_ring_clips(self, CircRingWall::Inner);

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		for (id, o) in outer_openings.iter().chain(inner_openings.iter()) {
			openings.insert(id.clone(), o.clone());
		}
		for (id, m) in outer_mapped.0.iter().chain(inner_mapped.0.iter()) {
			mapped.insert(id.clone(), *m);
		}

		let outer_wall = ClippedArcSweep::new(
			self.center_xz,
			self.outer_radius,
			self.storey_height,
			360.0,
			0.0,
			PartitionStyle::RoughStonework,
			outer_intervals,
		);
		// Inner ring: same kit orientation (outward from center = into the gallery).
		let inner_wall = ClippedArcSweep::new(
			self.center_xz,
			self.inner_radius,
			self.storey_height,
			360.0,
			0.0,
			PartitionStyle::RoughStonework,
			inner_intervals,
		);

		(outer_wall, inner_wall, openings, mapped)
	}
}

fn collect_ring_clips(
	params: &CircRingFloorParams,
	wall: CircRingWall,
) -> (Vec<(f32, f32)>, Openings, MappedOpenings) {
	let radius = match wall {
		CircRingWall::Outer => params.outer_radius,
		CircRingWall::Inner => params.inner_radius,
	};
	let mut candidates: Vec<(f32, f32, f32, OpeningId, Opening, MappedOpening)> = Vec::new();
	for (id, opening) in params.openings.iter() {
		if !opening.label.is_connectable() {
			continue;
		}
		if nearest_wall(params, &opening.bounds) != wall {
			continue;
		}
		let Some((t0, t1, mapped, score)) = ring_clip_t(params, radius, &opening.bounds) else {
			continue;
		};
		candidates.push((score, t0, t1, id.clone(), opening.clone(), mapped));
	}
	candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

	let mut intervals = Vec::new();
	let mut openings = Openings::new();
	let mut mapped = MappedOpenings::new();
	for (_score, t0, t1, id, opening, m) in candidates {
		if intervals.iter().any(|&(a, b)| intervals_overlap(a, b, t0, t1)) {
			continue;
		}
		intervals.push((t0, t1));
		mapped.insert(id.clone(), m);
		openings.insert(id, opening);
	}
	intervals.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
	(intervals, openings, mapped)
}

fn nearest_wall(params: &CircRingFloorParams, bounds: &Aabb3d) -> CircRingWall {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let r = Vec2::new(mid.x - params.center_xz.x, mid.z - params.center_xz.z).length();
	let d_outer = (r - params.outer_radius).abs();
	let d_inner = (r - params.inner_radius).abs();
	if d_inner < d_outer {
		CircRingWall::Inner
	} else {
		CircRingWall::Outer
	}
}

fn ring_clip_t(
	params: &CircRingFloorParams,
	radius: f32,
	bounds: &Aabb3d,
) -> Option<(f32, f32, MappedOpening, f32)> {
	let c = params.center_xz;
	let imin = Vec3::from(bounds.min);
	let imax = Vec3::from(bounds.max);
	let y0 = c.y;
	let y1 = y0 + params.storey_height;
	let hy0 = imin.y.clamp(y0, y1);
	let hy1 = imax.y.clamp(y0, y1);
	if hy1 - hy0 < EPS {
		return None;
	}

	let corners_xz = [
		Vec2::new(imin.x, imin.z),
		Vec2::new(imax.x, imin.z),
		Vec2::new(imin.x, imax.z),
		Vec2::new(imax.x, imax.z),
	];
	let c_xz = Vec2::new(c.x, c.z);
	let mut angles = Vec::new();
	for p in corners_xz {
		let d = p - c_xz;
		if d.length_squared() < 1e-8 {
			continue;
		}
		// Match arc_ring_dir: yaw such that (cos θ, −sin θ) ≈ dir ⇒ θ = atan2(−z, x).
		let yaw = (-d.y).atan2(d.x);
		let mut t = yaw / TAU;
		if t < 0.0 {
			t += 1.0;
		}
		angles.push(t);
	}
	if angles.is_empty() {
		return None;
	}

	let (t0, t1) = angular_span(&angles)?;
	let score = (t1 - t0) * radius * (hy1 - hy0);
	if score < EPS {
		return None;
	}

	let d0 = arc_ring_dir_deg(t0 * 360.0);
	let d1 = arc_ring_dir_deg(t1 * 360.0);
	let p0 = Vec3::new(c.x + d0.x * radius, hy0, c.z + d0.y * radius);
	let p1 = Vec3::new(c.x + d1.x * radius, hy0, c.z + d1.y * radius);
	let p2 = Vec3::new(c.x + d0.x * radius, hy1, c.z + d0.y * radius);
	let p3 = Vec3::new(c.x + d1.x * radius, hy1, c.z + d1.y * radius);
	let mid_t = 0.5 * (t0 + t1);
	let outward = arc_ring_dir_deg(mid_t * 360.0);
	let mapped = MappedOpening::new(MappedOpeningQuad::new(p0, p1, p2, p3), outward);
	Some((t0, t1, mapped, score))
}

/// Smallest arc covering the unit-circle samples in \(t \in [0, 1)\).
fn angular_span(ts: &[f32]) -> Option<(f32, f32)> {
	if ts.is_empty() {
		return None;
	}
	let mut sorted = ts.to_vec();
	for t in &mut sorted {
		*t = ((*t % 1.0) + 1.0) % 1.0;
	}
	sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	sorted.dedup_by(|a, b| (*a - *b).abs() < 1e-5);

	if sorted.len() == 1 {
		let t = sorted[0];
		return Some(((t - 0.04).clamp(0.0, 1.0), (t + 0.04).clamp(0.0, 1.0)));
	}

	// Largest gap ⇒ complement is the spanning arc.
	let mut max_gap = 0.0f32;
	let mut gap_after = 0usize;
	for i in 0..sorted.len() {
		let a = sorted[i];
		let b = if i + 1 < sorted.len() { sorted[i + 1] } else { sorted[0] + 1.0 };
		let gap = b - a;
		if gap > max_gap {
			max_gap = gap;
			gap_after = i;
		}
	}
	let t_start = if gap_after + 1 < sorted.len() { sorted[gap_after + 1] } else { sorted[0] };
	let t_end_raw = sorted[gap_after];
	let t_end = if t_end_raw < t_start - 1e-6 { t_end_raw + 1.0 } else { t_end_raw };

	// ClippedArcSweep clips do not wrap; keep the majority non-wrapping piece.
	if t_end > 1.0 {
		let wrap_hi = t_end - 1.0;
		if 1.0 - t_start >= wrap_hi {
			Some((t_start, 1.0))
		} else {
			Some((0.0, wrap_hi.max(EPS)))
		}
	} else if t_end - t_start < 0.04 {
		let mid = 0.5 * (t_start + t_end);
		Some(((mid - 0.04).clamp(0.0, 1.0), (mid + 0.04).clamp(0.0, 1.0)))
	} else {
		Some((t_start, t_end))
	}
}

fn intervals_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
	a0 < b1 - 1e-5 && b0 < a1 - 1e-5
}

impl MapsOpenings for CircRingFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}
