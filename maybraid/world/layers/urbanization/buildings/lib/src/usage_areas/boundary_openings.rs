//! Progressive [`OpeningLabel::Boundary`] authorship on plan edges.
//!
//! Used so one suite/primary-rect can wall a shared interface, then hand the
//! next neighbor a Boundary keep-out (no double wall). Exterior faces that the
//! shell already walls are also marked Boundary before fill.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};

use crate::fit::Confines;
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::usage_areas::plan_cells::shared_edge_span;

const EPS: f32 = 1e-3;

/// Insert a Boundary opening covering a plan edge span.
pub fn insert_boundary_edge(
	openings: &mut Openings,
	scope: &str,
	tag: impl AsRef<str>,
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
) {
	let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
	if hi - lo < EPS {
		return;
	}
	let half_d = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.06).max(0.12);
	let door_h = (y1 - y0).max(1.0);
	let bounds = if along_x {
		Aabb3d::from_min_max(
			Vec3::new(lo, y0, mid - half_d),
			Vec3::new(hi, y0 + door_h, mid + half_d),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(mid - half_d, y0, lo),
			Vec3::new(mid + half_d, y0 + door_h, hi),
		)
	};
	openings.insert(
		OpeningId::scoped(scope, "boundary", tag),
		Opening::new(bounds, OpeningLabel::Boundary),
	);
}

/// Mark primary-rect faces that are not shared with another primary as Boundary
/// (shell already owns the true exterior).
pub fn inject_exterior_boundaries(
	host: Aabb2d,
	siblings: &[Aabb2d],
	y0: f32,
	y1: f32,
	scope: &str,
	tag_prefix: &str,
	openings: &mut Openings,
) {
	let faces = host_face_spans(host);
	for (fi, (along_x, mid, lo, hi)) in faces.into_iter().enumerate() {
		let mut spans = vec![(lo, hi)];
		for sib in siblings {
			if let Some((sax, slo, shi, smid)) = shared_edge_span(host, *sib) {
				if sax == along_x && (smid - mid).abs() < 0.05 {
					spans = subtract_span(&spans, slo, shi);
				}
			}
		}
		for (si, (a, b)) in spans.into_iter().enumerate() {
			insert_boundary_edge(
				openings,
				scope,
				format!("{tag_prefix}_ext_{fi}_{si}"),
				along_x,
				a,
				b,
				mid,
				y0,
				y1,
			);
		}
	}
}

/// After packing `owner`, mark the shared edge on `neighbor` as Boundary.
pub fn inject_shared_boundary_from(
	owner: Aabb2d,
	neighbor: &mut Confines,
	scope: &str,
	tag: impl AsRef<str>,
) {
	let n = host_xz_of(neighbor);
	let Some((along_x, lo, hi, mid)) = shared_edge_span(owner, n) else {
		return;
	};
	let y0 = Vec3::from(neighbor.bounds.min).y;
	let y1 = Vec3::from(neighbor.bounds.max).y;
	insert_boundary_edge(&mut neighbor.openings, scope, tag, along_x, lo, hi, mid, y0, y1);
}

fn host_xz_of(c: &Confines) -> Aabb2d {
	let min = Vec3::from(c.bounds.min);
	let max = Vec3::from(c.bounds.max);
	Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) }
}

/// Four faces of an XZ host: `(along_x, mid_perp, lo_along, hi_along)`.
pub fn host_face_spans(host: Aabb2d) -> [(bool, f32, f32, f32); 4] {
	[
		(true, host.min.y, host.min.x, host.max.x),  // south (-Z)
		(true, host.max.y, host.min.x, host.max.x),  // north (+Z)
		(false, host.min.x, host.min.y, host.max.y), // west (-X)
		(false, host.max.x, host.min.y, host.max.y), // east (+X)
	]
}

/// True when a Boundary/Exclusion opening covers this plan edge span.
pub fn plan_edge_excluded(openings: &Openings, along_x: bool, lo: f32, hi: f32, mid: f32) -> bool {
	let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
	let span = (hi - lo).max(0.0);
	if span < EPS {
		return true;
	}
	let mut covered = 0.0_f32;
	for (_id, o) in openings.iter() {
		if !matches!(o.label, OpeningLabel::Boundary | OpeningLabel::Exclusion) {
			continue;
		}
		let omin = Vec3::from(o.bounds.min);
		let omax = Vec3::from(o.bounds.max);
		if along_x {
			// Constant Z = mid.
			if !(omin.z - 0.35 <= mid && mid <= omax.z + 0.35) {
				continue;
			}
			let clo = omin.x.max(lo);
			let chi = omax.x.min(hi);
			covered += (chi - clo).max(0.0);
		} else if omin.x - 0.35 <= mid && mid <= omax.x + 0.35 {
			let clo = omin.z.max(lo);
			let chi = omax.z.min(hi);
			covered += (chi - clo).max(0.0);
		}
	}
	covered + EPS >= span * 0.85
}

fn subtract_span(spans: &[(f32, f32)], cut_lo: f32, cut_hi: f32) -> Vec<(f32, f32)> {
	let (cut_lo, cut_hi) = if cut_lo <= cut_hi { (cut_lo, cut_hi) } else { (cut_hi, cut_lo) };
	let mut out = Vec::new();
	for &(lo, hi) in spans {
		if cut_hi <= lo + EPS || cut_lo >= hi - EPS {
			out.push((lo, hi));
			continue;
		}
		if cut_lo > lo + EPS {
			out.push((lo, cut_lo.min(hi)));
		}
		if cut_hi < hi - EPS {
			out.push((cut_hi.max(lo), hi));
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn exterior_leaves_shared_edge_unmarked() {
		let stem = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(4.0, 4.0) };
		let flange = Aabb2d { min: Vec2::new(0.0, 4.0), max: Vec2::new(10.0, 8.0) };
		let mut openings = Openings::new();
		inject_exterior_boundaries(stem, &[flange], 0.0, 3.0, "test", "stem", &mut openings);
		// Shared north edge of stem should NOT be fully Boundary-covered.
		assert!(!plan_edge_excluded(&openings, true, 0.0, 4.0, 4.0,));
		// South exterior should be covered.
		assert!(plan_edge_excluded(&openings, true, 0.0, 4.0, 0.0));
	}
}
