//! I-plan rectangles, outer wall edges, and slab pieces.

use bevy_math::{Vec2, Vec3};

use crate::shells::ortho::{PlanRect, WallEdge, EPS};

use super::IFloorParams;

/// Axis-aligned plan rectangle (full extents) for slabs / union / storey packing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanAabb {
	pub min_x: f32,
	pub max_x: f32,
	pub min_z: f32,
	pub max_z: f32,
}

impl PlanAabb {
	pub fn new(min_x: f32, max_x: f32, min_z: f32, max_z: f32) -> Self {
		Self {
			min_x: min_x.min(max_x),
			max_x: min_x.max(max_x),
			min_z: min_z.min(max_z),
			max_z: min_z.max(max_z),
		}
	}

	pub fn to_plan_rect(self, y: f32) -> PlanRect {
		PlanRect::new(
			Vec3::new(0.5 * (self.min_x + self.max_x), y, 0.5 * (self.min_z + self.max_z)),
			(self.max_x - self.min_x).max(EPS),
			(self.max_z - self.min_z).max(EPS),
		)
	}

	/// Plan footprint as [`bevy_math::bounding::Aabb2d`] (\(x → X\), \(y → Z\)).
	pub fn to_aabb2(self) -> bevy_math::bounding::Aabb2d {
		bevy_math::bounding::Aabb2d {
			min: Vec2::new(self.min_x, self.min_z),
			max: Vec2::new(self.max_x, self.max_z),
		}
	}

	pub fn width(self) -> f32 {
		(self.max_x - self.min_x).max(0.0)
	}

	pub fn depth(self) -> f32 {
		(self.max_z - self.min_z).max(0.0)
	}

	pub fn center_xz(self) -> Vec2 {
		Vec2::new(0.5 * (self.min_x + self.max_x), 0.5 * (self.min_z + self.max_z))
	}
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct IGeom {
	pub y0: f32,
	pub height: f32,
	pub slab_rects: Vec<PlanAabb>,
	pub edges: Vec<WallEdge>,
}

impl IFloorParams {
	pub(super) fn resolve_geometry(&self) -> IGeom {
		let cx = self.center_xz.x;
		let cz = self.center_xz.z;
		let y0 = self.center_xz.y;
		let height = self.storey_height.max(EPS);
		let w = self.central_rectangle.x.max(EPS);
		let d = self.central_rectangle.y.max(EPS);
		let half_w = w * 0.5;
		let half_d = d * 0.5;
		let flange_t = self.flange_thickness.map(|t| t.max(EPS)).unwrap_or(w);

		let tl = positive_len(self.top_left_length).unwrap_or(0.0);
		let tr = positive_len(self.top_right_length).unwrap_or(0.0);
		let bl = positive_len(self.bottom_left_length).unwrap_or(0.0);
		let br = positive_len(self.bottom_right_length).unwrap_or(0.0);
		// `Some(_)` means the flange bar is present (zero length ⇒ stem-width bar only).
		let has_top = self.top_left_length.is_some() || self.top_right_length.is_some();
		let has_bot = self.bottom_left_length.is_some() || self.bottom_right_length.is_some();

		let stem_x0 = cx - half_w;
		let stem_x1 = cx + half_w;
		let stem_z0 = cz - half_d;
		let stem_z1 = cz + half_d;
		let top_z1 = stem_z1 + flange_t;
		let bot_z0 = stem_z0 - flange_t;

		let mut slabs = vec![PlanAabb::new(stem_x0, stem_x1, stem_z0, stem_z1)];
		if has_top {
			slabs.push(PlanAabb::new(stem_x0 - tl, stem_x1 + tr, stem_z1, top_z1));
		}
		if has_bot {
			slabs.push(PlanAabb::new(stem_x0 - bl, stem_x1 + br, bot_z0, stem_z0));
		}

		let ring = outline_ring(
			stem_x0, stem_x1, stem_z0, stem_z1, top_z1, bot_z0, tl, tr, bl, br, has_top, has_bot,
		);
		let edges = ring_to_edges(&ring, y0, height);
		IGeom { y0, height, slab_rects: slabs, edges }
	}
}

fn positive_len(v: Option<f32>) -> Option<f32> {
	v.and_then(|x| (x > EPS).then_some(x))
}

/// CCW exterior ring in XZ.
fn outline_ring(
	stem_x0: f32,
	stem_x1: f32,
	stem_z0: f32,
	stem_z1: f32,
	top_z1: f32,
	bot_z0: f32,
	tl: f32,
	tr: f32,
	bl: f32,
	br: f32,
	has_top: bool,
	has_bot: bool,
) -> Vec<Vec2> {
	let mut pts = Vec::new();

	// --- bottom run ---
	if has_bot {
		pts.push(Vec2::new(stem_x0 - bl, bot_z0));
		pts.push(Vec2::new(stem_x1 + br, bot_z0));
		pts.push(Vec2::new(stem_x1 + br, stem_z0));
		if br > EPS {
			pts.push(Vec2::new(stem_x1, stem_z0));
		}
	} else {
		pts.push(Vec2::new(stem_x0, stem_z0));
		pts.push(Vec2::new(stem_x1, stem_z0));
	}

	// --- east stem up ---
	if has_top {
		pts.push(Vec2::new(stem_x1, stem_z1));
		if tr > EPS {
			pts.push(Vec2::new(stem_x1 + tr, stem_z1));
		}
		pts.push(Vec2::new(stem_x1 + tr, top_z1));
		pts.push(Vec2::new(stem_x0 - tl, top_z1));
		pts.push(Vec2::new(stem_x0 - tl, stem_z1));
		if tl > EPS {
			pts.push(Vec2::new(stem_x0, stem_z1));
		}
	} else {
		pts.push(Vec2::new(stem_x1, stem_z1));
		pts.push(Vec2::new(stem_x0, stem_z1));
	}

	// --- west stem down / bottom-left return ---
	if has_bot {
		pts.push(Vec2::new(stem_x0, stem_z0));
		if bl > EPS {
			pts.push(Vec2::new(stem_x0 - bl, stem_z0));
		}
	}

	dedup_ring(pts)
}

fn dedup_ring(pts: Vec<Vec2>) -> Vec<Vec2> {
	let mut out = Vec::new();
	for p in pts {
		if out.last().is_none_or(|q: &Vec2| q.distance(p) > EPS) {
			out.push(p);
		}
	}
	if out.len() >= 2 && out[0].distance(*out.last().unwrap()) < EPS {
		out.pop();
	}
	out
}

fn ring_to_edges(ring: &[Vec2], y: f32, height: f32) -> Vec<WallEdge> {
	if ring.len() < 3 {
		return Vec::new();
	}
	let n = ring.len();
	let mut edges = Vec::with_capacity(n);
	for i in 0..n {
		let a = ring[i];
		let b = ring[(i + 1) % n];
		if a.distance(b) < EPS {
			continue;
		}
		let start = Vec3::new(a.x, y, a.y);
		let end = Vec3::new(b.x, y, b.y);
		let along = (end - start).normalize_or_zero();
		let outward3 = Vec3::Y.cross(along);
		let outward = Vec2::new(outward3.x, outward3.z);
		let outward = if outward.length_squared() > 1e-8 { outward.normalize() } else { Vec2::X };
		edges.push(WallEdge::new(start, end, height, outward));
	}
	edges
}
