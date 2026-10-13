//! World-space triangle with one closed clip polyline → solid fill as [`PanelComplex`].
//!
//! The clip is projected onto the triangle plane ([`crate::paneling::panel_plane`]), intersected
//! with the outer triangle (Sutherland–Hodgman), then subtracted:
//! - **Interior hole** (strictly inside): bridge + ear-clip annulus.
//! - **Boundary-touching hole**: cancel shared edges and ear-clip the bite polygon.
//!
//! Present via [`Self::into_complex`]. Strip composition is future work.

use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelStyle;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPointId, DEFAULT_PANEL_THICKNESS,
};
use crate::paneling::panel_plane::{panel_plane_frame, PanelPlaneFrame};

/// Outer world triangle + one closed clip polyline → [`PanelComplex`] fill.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedTessellatedTriangle {
	pub style: PanelStyle,
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
	/// Closed clip (first connects to last). World positions; projected and clipped at build.
	pub clip: Vec<Vec3>,
	complex: PanelComplex,
}

impl ClippedTessellatedTriangle {
	/// Build eagerly. Degenerate outer → empty complex. Empty clip∩triangle → solid outer.
	pub fn new(
		style: PanelStyle,
		a: Vec3,
		b: Vec3,
		c: Vec3,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		let clip: Vec<Vec3> = clip.into_iter().map(Into::into).collect();
		let complex = build_complex(style, a, b, c, &clip, PanelComplexJointPolicy::default());
		Self { style, a, b, c, clip, complex }
	}

	pub fn rough_stone(
		a: Vec3,
		b: Vec3,
		c: Vec3,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a, b, c, clip)
	}

	pub fn shepherds_thatch(
		a: Vec3,
		b: Vec3,
		c: Vec3,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a, b, c, clip)
	}

	/// Rebuild with a new clip, keeping style / corners / joint policy.
	pub fn with_clip(self, clip: impl IntoIterator<Item = impl Into<Vec3>>) -> Self {
		let policy = self.complex.joint_policy;
		let mut next = Self::new(self.style, self.a, self.b, self.c, clip);
		next.complex = next.complex.with_joint_policy(policy);
		next
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.complex.set_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}
}

impl AsRef<PanelComplex> for ClippedTessellatedTriangle {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<ClippedTessellatedTriangle> for PanelComplex {
	fn from(value: ClippedTessellatedTriangle) -> Self {
		value.into_complex()
	}
}

fn build_complex(
	style: PanelStyle,
	a: Vec3,
	b: Vec3,
	c: Vec3,
	clip: &[Vec3],
	joint_policy: PanelComplexJointPolicy,
) -> PanelComplex {
	let mut complex = PanelComplex::new(style).with_joint_policy(joint_policy);
	let Some(frame) = panel_plane_frame(a, b, c) else {
		debug_assert!(false, "ClippedTessellatedTriangle: degenerate outer triangle");
		return complex;
	};

	let outer = orient_ccw(frame.outer_2d().to_vec());
	let clip_2d: Vec<Vec2> = clip.iter().copied().map(|p| frame.project(p)).collect();

	let fill_tris = match clip_and_triangulate(&outer, &clip_2d) {
		FillTris::SolidOuter => vec![[outer[0], outer[1], outer[2]]],
		FillTris::Empty => Vec::new(),
		FillTris::Tris(tris) => tris,
		FillTris::Failed => {
			debug_assert!(
				false,
				"ClippedTessellatedTriangle: failed to triangulate clip; solid fallback"
			);
			vec![[outer[0], outer[1], outer[2]]]
		}
	};

	add_tris_to_complex(&mut complex, &frame, &fill_tris);
	complex
}

enum FillTris {
	SolidOuter,
	Empty,
	Tris(Vec<[Vec2; 3]>),
	Failed,
}

fn clip_and_triangulate(outer: &[Vec2], clip: &[Vec2]) -> FillTris {
	if clip.len() < 3 || signed_area(clip).abs() < 1e-10 {
		return FillTris::SolidOuter;
	}

	let mut hole = sutherland_hodgman(clip, outer);
	hole = dedup_poly(hole);
	if hole.len() < 3 || signed_area(&hole).abs() < 1e-10 {
		// Missed the triangle entirely (or degenerate intersection).
		return FillTris::SolidOuter;
	}

	let hole_ccw = orient_ccw(hole);
	let outer_area = signed_area(outer).abs();
	let hole_area = signed_area(&hole_ccw).abs();
	if hole_area >= outer_area - 1e-5 {
		return FillTris::Empty;
	}

	let fill_polys = if hole_touches_boundary(&hole_ccw, outer) {
		match bite_polygons(outer, &hole_ccw) {
			Some(ps) => ps,
			None => return FillTris::Failed,
		}
	} else {
		let hole_cw = orient_cw(hole_ccw.clone());
		match bridge_outer_hole(outer, &hole_cw) {
			Some(p) => vec![p],
			None => return FillTris::Failed,
		}
	};

	let mut all_tris = Vec::new();
	for fill_poly in &fill_polys {
		match ear_clip(fill_poly) {
			Some(tris) => all_tris.extend(tris),
			None => return FillTris::Failed,
		}
	}
	if all_tris.is_empty() {
		FillTris::Empty
	} else {
		FillTris::Tris(all_tris)
	}
}

/// Sutherland–Hodgman: subject ∩ convex clip polygon (CCW).
fn sutherland_hodgman(subject: &[Vec2], clip_ccw: &[Vec2]) -> Vec<Vec2> {
	let mut output = subject.to_vec();
	let n = clip_ccw.len();
	for i in 0..n {
		let c0 = clip_ccw[i];
		let c1 = clip_ccw[(i + 1) % n];
		let input = std::mem::take(&mut output);
		if input.is_empty() {
			break;
		}
		let mut prev = input[input.len() - 1];
		for &curr in &input {
			let curr_in = inside_halfplane(curr, c0, c1);
			let prev_in = inside_halfplane(prev, c0, c1);
			if curr_in {
				if !prev_in {
					if let Some(x) = line_intersect(prev, curr, c0, c1) {
						output.push(x);
					}
				}
				output.push(curr);
			} else if prev_in {
				if let Some(x) = line_intersect(prev, curr, c0, c1) {
					output.push(x);
				}
			}
			prev = curr;
		}
	}
	output
}

fn inside_halfplane(p: Vec2, a: Vec2, b: Vec2) -> bool {
	// Inclusive left of a→b (CCW clip keeps interior).
	cross2(a, b, p) >= -1e-7
}

fn line_intersect(p0: Vec2, p1: Vec2, c0: Vec2, c1: Vec2) -> Option<Vec2> {
	let r = p1 - p0;
	let s = c1 - c0;
	let den = r.x * s.y - r.y * s.x;
	if den.abs() < 1e-14 {
		return None;
	}
	let qp = c0 - p0;
	let t = (qp.x * s.y - qp.y * s.x) / den;
	Some(p0 + r * t)
}

fn hole_touches_boundary(hole: &[Vec2], outer: &[Vec2]) -> bool {
	hole.iter().any(|&p| on_boundary(p, outer))
}

fn on_boundary(p: Vec2, outer: &[Vec2]) -> bool {
	let n = outer.len();
	for i in 0..n {
		if on_segment(outer[i], outer[(i + 1) % n], p) {
			return true;
		}
	}
	false
}

/// Fill polygon(s) for outer \ hole when hole ⊆ outer and touches ∂outer.
///
/// Outer edges (CCW) + hole edges (CW); split at shared verts; cancel reverse pairs;
/// trace **every** remaining loop. A boundary-touching hole that spans two outer
/// edges (e.g. a ground door that also crosses a triangle edge) can leave two
/// disconnected fill components — keeping only the first drops one side.
fn bite_polygons(outer: &[Vec2], hole_ccw: &[Vec2]) -> Option<Vec<Vec<Vec2>>> {
	let mut verts: Vec<Vec2> = Vec::new();
	for &p in outer.iter().chain(hole_ccw.iter()) {
		push_unique(&mut verts, p);
	}

	let mut edges: Vec<(Vec2, Vec2)> = Vec::new();
	let on = outer.len();
	for i in 0..on {
		push_split_edge(&mut edges, outer[i], outer[(i + 1) % on], &verts);
	}
	let hn = hole_ccw.len();
	for i in 0..hn {
		// CW around hole (= CCW around fill when walking the cutout).
		push_split_edge(&mut edges, hole_ccw[(i + 1) % hn], hole_ccw[i], &verts);
	}

	cancel_reverse_edges(&mut edges);
	if edges.is_empty() {
		return None;
	}

	let mut loops = Vec::new();
	while !edges.is_empty() {
		let mut loop_poly = trace_loop(&mut edges)?;
		loop_poly = dedup_poly(loop_poly);
		if loop_poly.len() < 3 {
			return None;
		}
		loops.push(orient_ccw(loop_poly));
	}
	Some(loops)
}

fn push_unique(verts: &mut Vec<Vec2>, p: Vec2) {
	if !verts.iter().any(|&q| near(q, p)) {
		verts.push(p);
	}
}

fn push_split_edge(edges: &mut Vec<(Vec2, Vec2)>, a: Vec2, b: Vec2, verts: &[Vec2]) {
	if near(a, b) {
		return;
	}
	let ab = b - a;
	let len2 = ab.length_squared();
	if len2 < 1e-16 {
		return;
	}
	let mut ts: Vec<(f32, Vec2)> = vec![(0.0, a), (1.0, b)];
	for &v in verts {
		if near(v, a) || near(v, b) {
			continue;
		}
		if on_segment(a, b, v) {
			let t = (v - a).dot(ab) / len2;
			ts.push((t, v));
		}
	}
	ts.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap_or(std::cmp::Ordering::Equal));
	for w in ts.windows(2) {
		let (p, q) = (w[0].1, w[1].1);
		if !near(p, q) {
			edges.push((p, q));
		}
	}
}

fn cancel_reverse_edges(edges: &mut Vec<(Vec2, Vec2)>) {
	let mut i = 0;
	while i < edges.len() {
		let (a, b) = edges[i];
		if let Some(j) = edges
			.iter()
			.enumerate()
			.skip(i + 1)
			.find(|(_, e)| near(e.0, b) && near(e.1, a))
			.map(|(j, _)| j)
		{
			edges.remove(j);
			edges.remove(i);
			continue;
		}
		i += 1;
	}
}

fn trace_loop(edges: &mut Vec<(Vec2, Vec2)>) -> Option<Vec<Vec2>> {
	let (start, mut curr_b) = edges.swap_remove(0);
	let mut path = vec![start, curr_b];
	for _ in 0..edges.len() + 4 {
		if near(curr_b, start) {
			path.pop(); // drop duplicate close
			return Some(path);
		}
		let idx = edges.iter().position(|(a, _)| near(*a, curr_b))?;
		let (_, next) = edges.swap_remove(idx);
		path.push(next);
		curr_b = next;
	}
	None
}

fn add_tris_to_complex(complex: &mut PanelComplex, frame: &PanelPlaneFrame, tris: &[[Vec2; 3]]) {
	let mut key_to_id: Vec<(u64, PanelPointId)> = Vec::new();
	let mut id_of = |complex: &mut PanelComplex, p: Vec2| -> PanelPointId {
		let key = pack_vec2_key(p);
		if let Some((_, id)) = key_to_id.iter().find(|(k, _)| *k == key) {
			return *id;
		}
		let id = complex.insert_point_thick(frame.unproject(p), DEFAULT_PANEL_THICKNESS);
		key_to_id.push((key, id));
		id
	};
	for tri in tris {
		let ia = id_of(complex, tri[0]);
		let ib = id_of(complex, tri[1]);
		let ic = id_of(complex, tri[2]);
		complex.add_triangle(ia, ib, ic);
	}
}

fn pack_vec2_key(p: Vec2) -> u64 {
	let xi = (p.x * 1e5).round() as i32 as u32;
	let yi = (p.y * 1e5).round() as i32 as u32;
	((xi as u64) << 32) | yi as u64
}

fn dedup_poly(mut poly: Vec<Vec2>) -> Vec<Vec2> {
	poly.dedup_by(|a, b| near(*a, *b));
	if poly.len() >= 2 && near(poly[0], *poly.last().unwrap()) {
		poly.pop();
	}
	poly
}

fn signed_area(poly: &[Vec2]) -> f32 {
	let n = poly.len();
	if n < 3 {
		return 0.0;
	}
	let mut a = 0.0;
	for i in 0..n {
		let p = poly[i];
		let q = poly[(i + 1) % n];
		a += p.x * q.y - q.x * p.y;
	}
	a * 0.5
}

fn orient_ccw(mut poly: Vec<Vec2>) -> Vec<Vec2> {
	if signed_area(&poly) < 0.0 {
		poly.reverse();
	}
	poly
}

fn orient_cw(mut poly: Vec<Vec2>) -> Vec<Vec2> {
	if signed_area(&poly) > 0.0 {
		poly.reverse();
	}
	poly
}

fn cross2(o: Vec2, a: Vec2, b: Vec2) -> f32 {
	(a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

fn point_in_triangle_strict(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let area = cross2(a, b, c);
	if area.abs() < 1e-12 {
		return false;
	}
	let ab = cross2(a, b, p);
	let bc = cross2(b, c, p);
	let ca = cross2(c, a, p);
	if area > 0.0 {
		ab > 1e-8 && bc > 1e-8 && ca > 1e-8
	} else {
		ab < -1e-8 && bc < -1e-8 && ca < -1e-8
	}
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let area = cross2(a, b, c);
	if area.abs() < 1e-12 {
		return false;
	}
	let ab = cross2(a, b, p);
	let bc = cross2(b, c, p);
	let ca = cross2(c, a, p);
	if area > 0.0 {
		ab >= -1e-8 && bc >= -1e-8 && ca >= -1e-8
	} else {
		ab <= 1e-8 && bc <= 1e-8 && ca <= 1e-8
	}
}

fn on_segment(a: Vec2, b: Vec2, p: Vec2) -> bool {
	p.x >= a.x.min(b.x) - 1e-7
		&& p.x <= a.x.max(b.x) + 1e-7
		&& p.y >= a.y.min(b.y) - 1e-7
		&& p.y <= a.y.max(b.y) + 1e-7
		&& cross2(a, b, p).abs() < 1e-5
}

fn segments_properly_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	let d1 = cross2(a, b, c);
	let d2 = cross2(a, b, d);
	let d3 = cross2(c, d, a);
	let d4 = cross2(c, d, b);
	((d1 > 1e-8 && d2 < -1e-8) || (d1 < -1e-8 && d2 > 1e-8))
		&& ((d3 > 1e-8 && d4 < -1e-8) || (d3 < -1e-8 && d4 > 1e-8))
}

fn segment_intersects_open(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	segments_properly_intersect(a, b, c, d)
}

fn bridge_outer_hole(outer: &[Vec2], hole: &[Vec2]) -> Option<Vec<Vec2>> {
	if outer.len() < 3 || hole.is_empty() {
		return None;
	}
	let (hj, &h) = hole.iter().enumerate().min_by(|(_, p), (_, q)| {
		p.x.partial_cmp(&q.x)
			.unwrap_or(std::cmp::Ordering::Equal)
			.then(p.y.partial_cmp(&q.y).unwrap_or(std::cmp::Ordering::Equal))
	})?;

	let oi = (0..outer.len()).find(|&i| bridge_visible(outer[i], h, outer, hole))?;

	let mut poly = Vec::with_capacity(outer.len() + hole.len() + 2);
	for k in 0..outer.len() {
		poly.push(outer[(oi + k) % outer.len()]);
	}
	poly.push(outer[oi]);
	for k in 0..hole.len() {
		poly.push(hole[(hj + k) % hole.len()]);
	}
	poly.push(hole[hj]);

	if signed_area(&poly) < 0.0 {
		poly.reverse();
	}
	Some(poly)
}

fn bridge_visible(o: Vec2, h: Vec2, outer: &[Vec2], hole: &[Vec2]) -> bool {
	if (o - h).length_squared() < 1e-12 {
		return false;
	}
	let mid = (o + h) * 0.5;
	if !point_in_triangle(mid, outer[0], outer[1], outer[2]) {
		return false;
	}
	if point_in_polygon(mid, hole) {
		return false;
	}
	for i in 0..hole.len() {
		let a = hole[i];
		let b = hole[(i + 1) % hole.len()];
		if endpoints_touch(o, h, a, b) {
			continue;
		}
		if segment_intersects_open(o, h, a, b) {
			return false;
		}
	}
	for i in 0..outer.len() {
		let a = outer[i];
		let b = outer[(i + 1) % outer.len()];
		if endpoints_touch(o, h, a, b) {
			continue;
		}
		if segment_intersects_open(o, h, a, b) {
			return false;
		}
	}
	true
}

fn endpoints_touch(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
	near(a, c) || near(a, d) || near(b, c) || near(b, d)
}

fn near(a: Vec2, b: Vec2) -> bool {
	(a - b).length_squared() < 1e-12
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
	let n = poly.len();
	if n < 3 {
		return false;
	}
	let mut inside = false;
	let mut j = n - 1;
	for i in 0..n {
		let pi = poly[i];
		let pj = poly[j];
		if on_segment(pj, pi, p) {
			return true;
		}
		let intersect = ((pi.y > p.y) != (pj.y > p.y))
			&& (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-30) + pi.x);
		if intersect {
			inside = !inside;
		}
		j = i;
	}
	inside
}

fn ear_clip(poly: &[Vec2]) -> Option<Vec<[Vec2; 3]>> {
	let mut verts = dedup_poly(poly.to_vec());
	if verts.len() < 3 {
		return None;
	}
	if signed_area(&verts) < 0.0 {
		verts.reverse();
	}

	let mut tris = Vec::new();
	let mut guard = 0;
	let max_guard = verts.len() * verts.len() + 32;
	while verts.len() > 3 {
		guard += 1;
		if guard > max_guard {
			return None;
		}
		let n = verts.len();
		let mut clipped = false;
		for i in 0..n {
			let prev = verts[(i + n - 1) % n];
			let curr = verts[i];
			let next = verts[(i + 1) % n];
			if !is_convex_ear(prev, curr, next) {
				continue;
			}
			if ear_contains_other(&verts, i, prev, curr, next) {
				continue;
			}
			tris.push([prev, curr, next]);
			verts.remove(i);
			clipped = true;
			break;
		}
		if !clipped {
			return None;
		}
	}
	tris.push([verts[0], verts[1], verts[2]]);
	Some(tris)
}

fn is_convex_ear(prev: Vec2, curr: Vec2, next: Vec2) -> bool {
	cross2(prev, curr, next) > 1e-8
}

fn ear_contains_other(verts: &[Vec2], ear_i: usize, a: Vec2, b: Vec2, c: Vec2) -> bool {
	let n = verts.len();
	for (j, &p) in verts.iter().enumerate() {
		if j == ear_i || j == (ear_i + n - 1) % n || j == (ear_i + 1) % n {
			continue;
		}
		if point_in_triangle_strict(p, a, b, c) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;

	fn ground_triangle() -> (Vec3, Vec3, Vec3) {
		(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0))
	}

	fn rect_clip() -> [Vec3; 4] {
		[
			Vec3::new(0.8, 0.0, 0.5),
			Vec3::new(1.4, 0.0, 0.5),
			Vec3::new(1.4, 0.0, 0.9),
			Vec3::new(0.8, 0.0, 0.9),
		]
	}

	#[test]
	fn interior_hole_leaves_centroid_empty() {
		let (a, b, c) = ground_triangle();
		let clip = rect_clip();
		let g = ClippedTessellatedTriangle::rough_stone(a, b, c, clip);
		assert!(g.as_complex().triangles().len() >= 3);
		let frame = panel_plane_frame(a, b, c).unwrap();
		let centroid =
			frame.project(clip.iter().copied().fold(Vec3::ZERO, |s, p| s + p) / clip.len() as f32);
		for tri in g.as_complex().triangles() {
			let pa = frame.project(g.as_complex().point(tri.a).unwrap().position);
			let pb = frame.project(g.as_complex().point(tri.b).unwrap().position);
			let pc = frame.project(g.as_complex().point(tri.c).unwrap().position);
			assert!(
				!point_in_triangle_strict(centroid, pa, pb, pc),
				"clip centroid should not lie in fill triangle"
			);
		}
	}

	#[test]
	fn oversized_trap_clips_across_boundary() {
		let (a, b, c) = ground_triangle();
		// Extends past CA and above BC — same authoring intent as the playground example.
		let clip = [
			Vec3::new(-0.4, 0.0, 0.3),
			Vec3::new(2.6, 0.0, 0.3),
			Vec3::new(1.8, 0.0, 1.4),
			Vec3::new(0.2, 0.0, 1.7),
		];
		let g = ClippedTessellatedTriangle::rough_stone(a, b, c, clip);
		let n = g.as_complex().triangles().len();
		assert!(n >= 2, "expected a bitten fill, got {n} tris");
		assert!(n < 20, "unexpectedly many tris: {n}");
		// A point deep in the clipped hole (inside ABC ∩ trap) should not be covered.
		let frame = panel_plane_frame(a, b, c).unwrap();
		let probe = frame.project(Vec3::new(0.8, 0.0, 0.6));
		for tri in g.as_complex().triangles() {
			let pa = frame.project(g.as_complex().point(tri.a).unwrap().position);
			let pb = frame.project(g.as_complex().point(tri.b).unwrap().position);
			let pc = frame.project(g.as_complex().point(tri.c).unwrap().position);
			assert!(
				!point_in_triangle_strict(probe, pa, pb, pc),
				"probe inside clip should not be covered by fill"
			);
		}
	}

	#[test]
	fn clip_outside_triangle_is_solid() {
		let (a, b, c) = ground_triangle();
		let clip = [
			Vec3::new(-2.0, 0.0, -1.0),
			Vec3::new(-1.0, 0.0, -1.0),
			Vec3::new(-1.0, 0.0, -0.5),
			Vec3::new(-2.0, 0.0, -0.5),
		];
		let g = ClippedTessellatedTriangle::rough_stone(a, b, c, clip);
		assert_eq!(g.as_complex().triangles().len(), 1);
	}

	#[test]
	fn short_clip_is_solid_outer() {
		let (a, b, c) = ground_triangle();
		let g = ClippedTessellatedTriangle::rough_stone(a, b, c, [Vec3::new(1.0, 0.0, 0.5)]);
		assert_eq!(g.as_complex().triangles().len(), 1);
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn degenerate_outer_yields_empty() {
		let g = ClippedTessellatedTriangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			rect_clip(),
		);
		assert!(g.as_complex().triangles().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "degenerate outer triangle")]
	fn degenerate_outer_debug_asserts() {
		let _ = ClippedTessellatedTriangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			rect_clip(),
		);
	}

	#[test]
	fn fill_has_shared_edges_and_never_policy_suppresses_joints() {
		let (a, b, c) = ground_triangle();
		let g = ClippedTessellatedTriangle::rough_stone(a, b, c, rect_clip());
		assert!(!g.as_complex().shared_edges().is_empty(), "annulus fill should share edges");
		let never = ClippedTessellatedTriangle::rough_stone(a, b, c, rect_clip())
			.with_joint_policy(PanelComplexJointPolicy::never())
			.into_complex();
		assert!(never.joint_nodes().is_empty());
		assert!(never.panel_nodes_for_level(LodSceneLevel::High).flatten().len() >= 3);
	}

	/// Boundary hole that splits outer \ hole into two components (ground notch
	/// spanning two edges of a right triangle).
	#[test]
	fn bite_keeps_both_fill_components() {
		// Right triangle CCW: (0,0)-(4,0)-(4,3). Door on the ground edge that
		// also crosses the hypotenuse → two fill loops.
		let outer = orient_ccw(vec![Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0), Vec2::new(4.0, 3.0)]);
		let clip =
			[Vec2::new(1.5, 0.0), Vec2::new(2.5, 0.0), Vec2::new(2.5, 2.1), Vec2::new(1.5, 2.1)];
		let hole = orient_ccw(dedup_poly(sutherland_hodgman(&clip, &outer)));
		assert!(hole_touches_boundary(&hole, &outer));
		let loops = bite_polygons(&outer, &hole).expect("bite");
		assert_eq!(loops.len(), 2, "expected left and right fill components");
		match clip_and_triangulate(&outer, &clip) {
			FillTris::Tris(tris) => {
				assert!(tris.len() >= 2);
				let covered =
					|p: Vec2| tris.iter().any(|t| point_in_triangle_strict(p, t[0], t[1], t[2]));
				assert!(covered(Vec2::new(0.5, 0.1)), "left remnant");
				assert!(covered(Vec2::new(3.7, 0.1)), "right remnant");
				assert!(!covered(Vec2::new(2.0, 0.5)), "door interior");
			}
			_ => panic!("expected Tris, got non-tris fill"),
		}
	}
}
