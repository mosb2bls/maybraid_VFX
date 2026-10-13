//! Shared geometric predicates for the layout trier.

use bevy_math::bounding::Aabb2d;
use procedural_common::{intersects_aabb2, touches_aabb2};

use crate::usage_areas::clearance::approach_blocked;

use super::kind::Predicate;

/// Inputs for evaluating a candidate plan footprint.
#[derive(Debug, Clone, Copy)]
pub struct PredicateCtx<'a> {
	pub host: Aabb2d,
	pub candidate: Aabb2d,
	pub clearances: &'a [Aabb2d],
	/// Door keep-out when testing [`Predicate::ApproachFree`]; ignored otherwise.
	pub door_clear: Option<Aabb2d>,
	/// Epsilon for wall-touch tests (m).
	pub wall_eps: f32,
}

/// Candidate fully inside the host (with a tiny inward margin).
pub fn in_host(host: Aabb2d, candidate: Aabb2d, margin: f32) -> bool {
	candidate.min.x >= host.min.x - margin
		&& candidate.min.y >= host.min.y - margin
		&& candidate.max.x <= host.max.x + margin
		&& candidate.max.y <= host.max.y + margin
}

/// Candidate does not intersect any keep-out.
pub fn clear_of_keep_outs(candidate: Aabb2d, clearances: &[Aabb2d]) -> bool {
	!clearances.iter().any(|c| intersects_aabb2(candidate, *c))
}

/// Candidate touches at least one host wall (plan edge).
pub fn against_wall(host: Aabb2d, candidate: Aabb2d, eps: f32) -> bool {
	let touch_min_x = (candidate.min.x - host.min.x).abs() <= eps;
	let touch_max_x = (candidate.max.x - host.max.x).abs() <= eps;
	let touch_min_z = (candidate.min.y - host.min.y).abs() <= eps;
	let touch_max_z = (candidate.max.y - host.max.y).abs() <= eps;
	touch_min_x || touch_max_x || touch_min_z || touch_max_z
}

/// Longer plan face lies on a host wall.
pub fn long_face_on_wall(host: Aabb2d, candidate: Aabb2d, eps: f32) -> bool {
	let dx = candidate.max.x - candidate.min.x;
	let dz = candidate.max.y - candidate.min.y;
	if dx >= dz {
		let touch_min_z = (candidate.min.y - host.min.y).abs() <= eps;
		let touch_max_z = (candidate.max.y - host.max.y).abs() <= eps;
		touch_min_z || touch_max_z
	} else {
		let touch_min_x = (candidate.min.x - host.min.x).abs() <= eps;
		let touch_max_x = (candidate.max.x - host.max.x).abs() <= eps;
		touch_min_x || touch_max_x
	}
}

/// Padded door approach is free of existing clearances.
pub fn approach_free(door_clear: Aabb2d, clearances: &[Aabb2d]) -> bool {
	!approach_blocked(door_clear, clearances)
}

/// Run every predicate in `list`; all must pass.
pub fn all_pass(list: &[Predicate], ctx: PredicateCtx<'_>) -> bool {
	list.iter().all(|p| match p {
		Predicate::InHost => in_host(ctx.host, ctx.candidate, 1e-3),
		Predicate::ClearOfKeepOuts => clear_of_keep_outs(ctx.candidate, ctx.clearances),
		Predicate::AgainstWall => against_wall(ctx.host, ctx.candidate, ctx.wall_eps),
		Predicate::LongFaceOnWall => long_face_on_wall(ctx.host, ctx.candidate, ctx.wall_eps),
		Predicate::ApproachFree => match ctx.door_clear {
			Some(d) => approach_free(d, ctx.clearances),
			None => true,
		},
	})
}

/// Convenience: candidate abuts another AABB with a small gap (nightstand / bed).
pub fn abuts_with_gap(a: Aabb2d, b: Aabb2d, gap: f32) -> bool {
	touches_aabb2(a, inflate_for_gap(b, gap)) && !intersects_aabb2(a, b)
}

fn inflate_for_gap(b: Aabb2d, gap: f32) -> Aabb2d {
	use procedural_common::inflate_aabb2;
	inflate_aabb2(b, gap)
}
