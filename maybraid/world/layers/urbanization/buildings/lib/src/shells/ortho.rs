//! Shared helpers for orthonormal storey shells (positioned opening fit).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};

use crate::openings::{MappedOpening, MappedOpeningQuad};
use crate::paneling::rect_fit::RectInset;

pub const EPS: f32 = 1e-4;
/// Thin slab volume half-height for intersection tests.
pub const SLAB_Y_HALF: f32 = 0.2;

/// Axis-aligned plan rectangle at elevation `y`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanRect {
	pub y: f32,
	pub half_x: f32,
	pub half_z: f32,
	pub center: Vec3,
}

impl PlanRect {
	pub fn new(center: Vec3, full_x: f32, full_z: f32) -> Self {
		Self {
			y: center.y,
			half_x: (full_x * 0.5).max(EPS),
			half_z: (full_z * 0.5).max(EPS),
			center: Vec3::new(center.x, center.y, center.z),
		}
	}

	pub fn sw(self) -> Vec3 {
		Vec3::new(self.center.x - self.half_x, self.y, self.center.z - self.half_z)
	}
	pub fn se(self) -> Vec3 {
		Vec3::new(self.center.x + self.half_x, self.y, self.center.z - self.half_z)
	}
	pub fn ne(self) -> Vec3 {
		Vec3::new(self.center.x + self.half_x, self.y, self.center.z + self.half_z)
	}
	pub fn nw(self) -> Vec3 {
		Vec3::new(self.center.x - self.half_x, self.y, self.center.z + self.half_z)
	}

	pub fn full_x(self) -> f32 {
		self.half_x * 2.0
	}

	pub fn full_z(self) -> f32 {
		self.half_z * 2.0
	}

	pub fn min_xz(self) -> Vec2 {
		Vec2::new(self.center.x - self.half_x, self.center.z - self.half_z)
	}

	pub fn max_xz(self) -> Vec2 {
		Vec2::new(self.center.x + self.half_x, self.center.z + self.half_z)
	}

	pub fn volume_aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(
			Vec3::new(
				self.center.x - self.half_x,
				self.y - SLAB_Y_HALF,
				self.center.z - self.half_z,
			),
			Vec3::new(
				self.center.x + self.half_x,
				self.y + SLAB_Y_HALF,
				self.center.z + self.half_z,
			),
		)
	}
}

/// Cardinal face of a rectangular plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrthoSide {
	South = 0,
	East = 1,
	North = 2,
	West = 3,
}

impl OrthoSide {
	pub fn all() -> [Self; 4] {
		[Self::South, Self::East, Self::North, Self::West]
	}

	/// Face index matching [`RectangularNTube`] corners SW→SE→NE→NW.
	pub fn face_index(self) -> usize {
		self as usize
	}

	pub fn from_face_index(i: usize) -> Option<Self> {
		match i {
			0 => Some(Self::South),
			1 => Some(Self::East),
			2 => Some(Self::North),
			3 => Some(Self::West),
			_ => None,
		}
	}

	pub fn outward(self) -> Vec3 {
		match self {
			Self::South => -Vec3::Z,
			Self::East => Vec3::X,
			Self::North => Vec3::Z,
			Self::West => -Vec3::X,
		}
	}

	pub fn orientation(self) -> Vec2 {
		let o = self.outward();
		Vec2::new(o.x, o.z)
	}
}

/// One standing wall segment in plan (edge along the wall run at floor elevation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallEdge {
	pub start: Vec3,
	pub end: Vec3,
	pub height: f32,
	/// Outward horizontal unit in XZ.
	pub outward: Vec2,
}

impl WallEdge {
	pub fn new(start: Vec3, end: Vec3, height: f32, outward: Vec2) -> Self {
		Self { start, end, height: height.max(EPS), outward }
	}

	pub fn length(self) -> f32 {
		self.start.distance(self.end).max(EPS)
	}

	pub fn mid(self) -> Vec3 {
		(self.start + self.end) * 0.5
	}

	/// Along-wall unit from start → end.
	pub fn tangent(self) -> Vec3 {
		let d = self.end - self.start;
		let len = d.length();
		if len < EPS {
			Vec3::X
		} else {
			d / len
		}
	}
}

/// Face-local opening rectangle on a standing wall strip (`roll = 0`).
///
/// For [`ClippedRectangularStrip`] / [`Rectangle`]: kit width = height (vertical),
/// kit depth = wall length. [`RectInset`] uses left/right along vertical and
/// bottom/top along the wall run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceOpening {
	pub inset: RectInset,
	pub mapped: MappedOpening,
}

/// Project opening AABB onto a standing strip edge → positioned inset + mapped quad.
///
/// Returns [`None`] when the opening does not intersect the wall volume meaningfully.
pub fn standing_face_opening(
	edge: WallEdge,
	bounds: &Aabb3d,
	thickness: f32,
) -> Option<FaceOpening> {
	let len = edge.length();
	let h = edge.height;
	let tang = edge.tangent();
	let outward3 = Vec3::new(edge.outward.x, 0.0, edge.outward.y);
	let half_t = thickness.max(EPS) * 0.5 + 0.25;

	// Wall volume for intersection.
	let corners = [
		edge.start - outward3 * half_t,
		edge.start + outward3 * half_t,
		edge.end - outward3 * half_t,
		edge.end + outward3 * half_t,
		edge.start - outward3 * half_t + Vec3::Y * h,
		edge.end + outward3 * half_t + Vec3::Y * h,
	];
	let mut wmin = corners[0];
	let mut wmax = corners[0];
	for c in &corners[1..] {
		wmin = wmin.min(*c);
		wmax = wmax.max(*c);
	}
	let wall_aabb = Aabb3d::from_min_max(wmin, wmax);
	let inter = aabb_intersection(bounds, &wall_aabb)?;

	// Parameterize along wall (s) and height (y).
	let imin = Vec3::from(inter.min);
	let imax = Vec3::from(inter.max);
	let project_s = |p: Vec3| (p - edge.start).dot(tang);
	let inter_corners = [
		Vec3::new(imin.x, imin.y, imin.z),
		Vec3::new(imax.x, imin.y, imin.z),
		Vec3::new(imin.x, imin.y, imax.z),
		Vec3::new(imax.x, imin.y, imax.z),
		Vec3::new(imin.x, imax.y, imin.z),
		Vec3::new(imax.x, imax.y, imin.z),
		Vec3::new(imin.x, imax.y, imax.z),
		Vec3::new(imax.x, imax.y, imax.z),
	];
	let mut s_lo = len;
	let mut s_hi: f32 = 0.0;
	for c in inter_corners {
		let s = project_s(c).clamp(0.0, len);
		s_lo = s_lo.min(s);
		s_hi = s_hi.max(s);
	}
	let y0 = imin.y.max(edge.start.y);
	let y1 = imax.y.min(edge.start.y + h);
	if s_hi - s_lo < EPS || y1 - y0 < EPS {
		return None;
	}

	// Standing strip: width=height (e0/+Y), depth=length (e1/along edge).
	let left = (y0 - edge.start.y).clamp(0.0, h);
	let right = ((edge.start.y + h) - y1).clamp(0.0, h);
	let bottom = s_lo.clamp(0.0, len);
	let top = (len - s_hi).clamp(0.0, len);
	let inset = RectInset::new(left, right, bottom, top);
	if inset.is_solid() {
		return None;
	}

	let bl = edge.start + tang * s_lo + Vec3::Y * (y0 - edge.start.y);
	let br = edge.start + tang * s_hi + Vec3::Y * (y0 - edge.start.y);
	let tl = edge.start + tang * s_lo + Vec3::Y * (y1 - edge.start.y);
	let tr = edge.start + tang * s_hi + Vec3::Y * (y1 - edge.start.y);
	// Looking out: lower-left / lower-right with outward normal.
	let mapped = MappedOpening::new(MappedOpeningQuad::new(br, bl, tr, tl), edge.outward);

	Some(FaceOpening { inset, mapped })
}

/// NTube face opening: lowest edge vertical (floor→ceiling), height along plan.
///
/// [`RectInset`] left/right along the plan run, bottom/top along height.
pub fn ntube_face_opening(
	a_floor: Vec3,
	b_floor: Vec3,
	a_ceil: Vec3,
	bounds: &Aabb3d,
	thickness: f32,
) -> Option<FaceOpening> {
	let height = a_floor.distance(a_ceil).max(EPS);
	let len = a_floor.distance(b_floor).max(EPS);
	let along = (b_floor - a_floor) / len;
	let up = (a_ceil - a_floor) / height;
	// Outward for CCW plan SW→SE→NE→NW: up × along (path×up would point inward).
	let outward3 = up.cross(along).normalize_or_zero();
	let outward = Vec2::new(outward3.x, outward3.z);
	let edge = WallEdge::new(a_floor, b_floor, height, outward);

	// Reuse standing projection in edge space, then remap inset axes for NTube.
	let standing = standing_face_opening(edge, bounds, thickness)?;
	// standing: left/right vertical, bottom/top along wall
	// ntube: left/right along wall, bottom/top vertical
	let inset = RectInset::new(
		standing.inset.bottom,
		standing.inset.top,
		standing.inset.left,
		standing.inset.right,
	);
	Some(FaceOpening { inset, mapped: standing.mapped })
}

/// Face-aligned extent score (width × height on the face).
pub fn face_extent_score(bounds: &Aabb3d, along_horizontal: bool) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	let width = if along_horizontal { e.x } else { e.z };
	width.max(0.0) * e.y.max(0.0)
}

pub fn best_side_for_bounds(bounds: &Aabb3d, plan: PlanRect) -> OrthoSide {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let candidates = [
		(OrthoSide::South, Vec3::new(plan.center.x, mid.y, plan.center.z - plan.half_z)),
		(OrthoSide::East, Vec3::new(plan.center.x + plan.half_x, mid.y, plan.center.z)),
		(OrthoSide::North, Vec3::new(plan.center.x, mid.y, plan.center.z + plan.half_z)),
		(OrthoSide::West, Vec3::new(plan.center.x - plan.half_x, mid.y, plan.center.z)),
	];
	candidates
		.into_iter()
		.min_by(|(_, a), (_, b)| {
			mid.distance_squared(*a)
				.partial_cmp(&mid.distance_squared(*b))
				.unwrap_or(std::cmp::Ordering::Equal)
		})
		.map(|(side, _)| side)
		.unwrap_or(OrthoSide::South)
}

/// Positioned hole inset for a horizontal fitted rectangle (a0→a1 along +X-ish, a0→b0 along +Z-ish).
///
/// Returns [`None`] when the opening misses the slab; `Some(None)` when it covers the slab
/// (caller should omit the piece); `Some(Some(inset))` for a positioned hole.
pub fn horizontal_slab_inset(plan: PlanRect, bounds: &Aabb3d) -> Option<Option<RectInset>> {
	let cut = horizontal_slab_cut_xz(plan, bounds)?;
	let pmin = plan.min_xz();
	let pmax = plan.max_xz();
	let full_x = plan.full_x();
	let full_z = plan.full_z();
	let x0 = cut.min.x;
	let x1 = cut.max.x;
	let z0 = cut.min.y;
	let z1 = cut.max.y;
	// Coverage: hole ate the fill.
	if (x1 - x0) + EPS >= full_x && (z1 - z0) + EPS >= full_z {
		return Some(None);
	}
	// Fitted rect: a0=SW, a1=SE (+X), b0=NW (+Z) → width=+X, depth=+Z.
	let left = (x0 - pmin.x).clamp(0.0, full_x);
	let right = (pmax.x - x1).clamp(0.0, full_x);
	let bottom = (z0 - pmin.y).clamp(0.0, full_z);
	let top = (pmax.y - z1).clamp(0.0, full_z);
	let inset = RectInset::new(left, right, bottom, top);
	if inset.is_solid() {
		return None;
	}
	Some(Some(inset))
}

/// Merge several slab-cutting insets: keep the largest-area hole, or remove if any removes.
///
/// Prefer [`subtract_aabb2d`] + residual solids when a band may host multiple
/// holes (e.g. two corner shafts on one N/S gallery strip). This helper remains
/// for shells that still author a single framed inset per piece.
pub fn merge_slab_insets(
	plan: PlanRect,
	openings: impl Iterator<Item = Aabb3d>,
) -> Option<Option<RectInset>> {
	let mut best: Option<RectInset> = None;
	let mut best_area = 0.0f32;
	for bounds in openings {
		match horizontal_slab_inset(plan, &bounds) {
			None => {}
			Some(None) => return Some(None),
			Some(Some(inset)) => {
				let w = plan.full_x() - inset.left - inset.right;
				let d = plan.full_z() - inset.bottom - inset.top;
				let area = w.max(0.0) * d.max(0.0);
				if area > best_area {
					best_area = area;
					best = Some(inset);
				}
			}
		}
	}
	best.map(Some)
}

/// Plan footprint of [`PlanRect`] as XZ → [`Aabb2d`] (`y` = world Z).
pub fn plan_rect_aabb2(plan: PlanRect) -> Aabb2d {
	Aabb2d { min: plan.min_xz(), max: plan.max_xz() }
}

/// Intersection of a 3D opening with the slab volume, as an XZ cut rectangle.
pub fn horizontal_slab_cut_xz(plan: PlanRect, bounds: &Aabb3d) -> Option<Aabb2d> {
	let slab = plan.volume_aabb();
	let inter = aabb_intersection(bounds, &slab)?;
	let imin = Vec3::from(inter.min);
	let imax = Vec3::from(inter.max);
	let pmin = plan.min_xz();
	let pmax = plan.max_xz();
	let x0 = imin.x.clamp(pmin.x, pmax.x);
	let x1 = imax.x.clamp(pmin.x, pmax.x);
	let z0 = imin.z.clamp(pmin.y, pmax.y);
	let z1 = imax.z.clamp(pmin.y, pmax.y);
	if x1 - x0 < EPS || z1 - z0 < EPS {
		return None;
	}
	Some(Aabb2d { min: Vec2::new(x0, z0), max: Vec2::new(x1, z1) })
}

fn aabb2_area(r: Aabb2d) -> f32 {
	(r.max.x - r.min.x).max(0.0) * (r.max.y - r.min.y).max(0.0)
}

fn aabb2_covers(host: Aabb2d, cut: Aabb2d) -> bool {
	cut.min.x <= host.min.x + EPS
		&& cut.min.y <= host.min.y + EPS
		&& cut.max.x + EPS >= host.max.x
		&& cut.max.y + EPS >= host.max.y
}

/// Subtract axis-aligned `cuts` from `host`, returning residual rectangles.
///
/// Guillotine difference against each cut in order — same idea as
/// `usage_areas::plan_cells::subtract_aabb2`, kept local so shells do not depend
/// on usage areas. Early-outs when any cut fully covers the host.
pub fn subtract_aabb2d(host: Aabb2d, cuts: &[Aabb2d]) -> Vec<Aabb2d> {
	if cuts.iter().any(|c| aabb2_covers(host, *c)) {
		return Vec::new();
	}
	let mut regions = vec![host];
	for cut in cuts {
		let mut next = Vec::new();
		for r in regions {
			next.extend(subtract_aabb2d_one(r, *cut));
		}
		regions = next;
	}
	regions.into_iter().filter(|r| aabb2_area(*r) > EPS * EPS).collect()
}

fn subtract_aabb2d_one(host: Aabb2d, cut: Aabb2d) -> Vec<Aabb2d> {
	let x0 = host.min.x.max(cut.min.x);
	let x1 = host.max.x.min(cut.max.x);
	let y0 = host.min.y.max(cut.min.y);
	let y1 = host.max.y.min(cut.max.y);
	if x1 - x0 <= EPS || y1 - y0 <= EPS {
		return vec![host];
	}
	let mut out = Vec::new();
	if x0 - host.min.x > EPS {
		out.push(Aabb2d { min: host.min, max: Vec2::new(x0, host.max.y) });
	}
	if host.max.x - x1 > EPS {
		out.push(Aabb2d { min: Vec2::new(x1, host.min.y), max: host.max });
	}
	if y0 - host.min.y > EPS {
		out.push(Aabb2d { min: Vec2::new(x0, host.min.y), max: Vec2::new(x1, y0) });
	}
	if host.max.y - y1 > EPS {
		out.push(Aabb2d { min: Vec2::new(x0, y1), max: Vec2::new(x1, host.max.y) });
	}
	out
}

/// Rebuild a [`PlanRect`] at elevation `y` from an XZ residual (`Aabb2d.y` = Z).
pub fn plan_rect_from_aabb2(y: f32, region: Aabb2d) -> PlanRect {
	PlanRect::new(
		Vec3::new(0.5 * (region.min.x + region.max.x), y, 0.5 * (region.min.y + region.max.y)),
		(region.max.x - region.min.x).max(EPS),
		(region.max.y - region.min.y).max(EPS),
	)
}

pub fn aabb_intersection(a: &Aabb3d, b: &Aabb3d) -> Option<Aabb3d> {
	if !aabb3d_intersects(a, b) {
		return None;
	}
	let min = Vec3::from(a.min).max(Vec3::from(b.min));
	let max = Vec3::from(a.max).min(Vec3::from(b.max));
	Some(Aabb3d::from_min_max(min, max))
}

pub fn aabb3d_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - EPS
		&& a.max.x > b.min.x + EPS
		&& a.min.y < b.max.y - EPS
		&& a.max.y > b.min.y + EPS
		&& a.min.z < b.max.z - EPS
		&& a.max.z > b.min.z + EPS
}

/// Nearest wall-edge index by midpoint distance; returns score for winner selection.
pub fn edge_score_for_bounds(bounds: &Aabb3d, edge: WallEdge) -> (f32, f32) {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let dist = mid.distance_squared(edge.mid());
	let along_x = edge.tangent().x.abs() > edge.tangent().z.abs();
	let score = face_extent_score(bounds, along_x);
	(dist, score)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn standing_inset_matches_offset_aabb() {
		let edge = WallEdge::new(
			Vec3::new(-4.0, 0.0, -3.0),
			Vec3::new(4.0, 0.0, -3.0),
			3.0,
			Vec2::new(0.0, -1.0),
		);
		// Door toward +X side, ground to 2.0.
		let bounds = Aabb3d::from_min_max(Vec3::new(1.0, 0.0, -3.2), Vec3::new(2.5, 2.0, -2.8));
		let face = standing_face_opening(edge, &bounds, 0.75).unwrap();
		assert!(face.inset.left < 0.05, "ground door left={}", face.inset.left);
		assert!((face.inset.right - 1.0).abs() < 0.15);
		assert!(face.inset.bottom > 4.0, "offset along wall");
		assert!(face.mapped.orientation.y < -0.9);
	}

	#[test]
	fn slab_inset_is_positioned_not_centered() {
		let plan = PlanRect::new(Vec3::ZERO, 8.0, 6.0);
		let bounds = Aabb3d::from_min_max(Vec3::new(1.0, -0.2, -2.0), Vec3::new(3.0, 0.2, -0.5));
		let Some(Some(inset)) = horizontal_slab_inset(plan, &bounds) else {
			panic!("expected positioned hole");
		};
		assert!(inset.left > inset.right, "hole on +X side");
		assert!(inset.bottom < inset.top, "hole on -Z side");
	}

	#[test]
	fn subtract_aabb2d_keeps_both_side_holes() {
		let host = Aabb2d { min: Vec2::new(-4.0, -3.0), max: Vec2::new(4.0, -1.5) };
		let cuts = [
			Aabb2d { min: Vec2::new(-4.0, -3.0), max: Vec2::new(-2.0, -1.5) },
			Aabb2d { min: Vec2::new(2.0, -3.0), max: Vec2::new(4.0, -1.5) },
		];
		let residuals = subtract_aabb2d(host, &cuts);
		assert_eq!(residuals.len(), 1);
		let mid = residuals[0];
		assert!((mid.min.x - (-2.0)).abs() < 1e-4);
		assert!((mid.max.x - 2.0).abs() < 1e-4);
	}
}
