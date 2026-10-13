//! Plan rectangles, face rails, and high-LOD posts.

use bevy_math::{Vec2, Vec3};

use super::TrazaloidParams;

pub(super) const EXTENT_EPS: f32 = 1e-3;
pub(super) const GAP_EPS: f32 = 1e-4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PlanRect {
	pub y: f32,
	pub cx: f32,
	pub cz: f32,
	pub half_x: f32,
	pub half_z: f32,
}

impl PlanRect {
	pub fn sw(self) -> Vec3 {
		Vec3::new(self.cx - self.half_x, self.y, self.cz - self.half_z)
	}
	pub fn se(self) -> Vec3 {
		Vec3::new(self.cx + self.half_x, self.y, self.cz - self.half_z)
	}
	pub fn ne(self) -> Vec3 {
		Vec3::new(self.cx + self.half_x, self.y, self.cz + self.half_z)
	}
	pub fn nw(self) -> Vec3 {
		Vec3::new(self.cx - self.half_x, self.y, self.cz + self.half_z)
	}

	pub fn full_x(self) -> f32 {
		self.half_x * 2.0
	}

	pub fn full_z(self) -> f32 {
		self.half_z * 2.0
	}
}

/// Cardinal face of a [`super::Trazaloid`] (lower / upper band).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrazaloidSide {
	North,
	East,
	South,
	West,
}

impl TrazaloidSide {
	pub fn all() -> [Self; 4] {
		[Self::North, Self::East, Self::South, Self::West]
	}

	/// Outward horizontal unit normal in XZ.
	pub fn outward(self) -> Vec3 {
		match self {
			Self::North => Vec3::Z,
			Self::East => Vec3::X,
			Self::South => -Vec3::Z,
			Self::West => -Vec3::X,
		}
	}

	/// Outward facing in plan (\(x, z\)).
	pub fn orientation(self) -> Vec2 {
		let o = self.outward();
		Vec2::new(o.x, o.z)
	}
}

/// One vertical post segment for high-LOD joint emission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PostSegment {
	pub start: Vec3,
	pub end: Vec3,
	pub radial_hint: Vec3,
}

impl TrazaloidParams {
	pub(super) fn resolve_rects(&self) -> [PlanRect; 4] {
		let foot_x = self.footprint.x.max(EXTENT_EPS) * 0.5;
		let foot_z = self.footprint.y.max(EXTENT_EPS) * 0.5;
		let ridge_x = self.ridge.x.max(EXTENT_EPS) * 0.5;
		let ridge_z = self.ridge.y.max(EXTENT_EPS) * 0.5;
		let lower_h = self.lower_height.max(EXTENT_EPS);
		let upper_h = self.upper_height.max(EXTENT_EPS);
		let gap = self.band_vertical_offset.max(0.0);
		let total = lower_h + gap + upper_h;
		let t = (lower_h / total).clamp(0.0, 1.0);

		let silhouette_x = foot_x + (ridge_x - foot_x) * t;
		let silhouette_z = foot_z + (ridge_z - foot_z) * t;
		let inset = self.waist_horizontal_offset.max(0.0);
		let waist_x = (silhouette_x - inset).max(EXTENT_EPS);
		let waist_z = (silhouette_z - inset).max(EXTENT_EPS);
		let origin = self.origin;

		[
			PlanRect { y: origin.y, cx: origin.x, cz: origin.z, half_x: foot_x, half_z: foot_z },
			PlanRect {
				y: origin.y + lower_h,
				cx: origin.x,
				cz: origin.z,
				half_x: waist_x,
				half_z: waist_z,
			},
			PlanRect {
				y: origin.y + lower_h + gap,
				cx: origin.x,
				cz: origin.z,
				half_x: waist_x,
				half_z: waist_z,
			},
			PlanRect {
				y: origin.y + lower_h + gap + upper_h,
				cx: origin.x,
				cz: origin.z,
				half_x: ridge_x,
				half_z: ridge_z,
			},
		]
	}

	pub(super) fn build_high_posts(&self, rects: &[PlanRect; 4]) -> Vec<PostSegment> {
		let [foot, waist, upper_bot, ridge] = *rects;
		let gap = upper_bot.y - waist.y;
		let mut posts = Vec::new();

		let corners = |r: PlanRect| [r.sw(), r.se(), r.ne(), r.nw()];
		let foot_c = corners(foot);
		let waist_c = corners(waist);
		let upper_c = corners(upper_bot);
		let ridge_c = corners(ridge);

		for i in 0..4 {
			let radial = (foot_c[i] - Vec3::new(self.origin.x, foot_c[i].y, self.origin.z))
				.normalize_or_zero();
			let radial = if radial.length_squared() > 0.0 { radial } else { Vec3::X };
			posts.push(PostSegment { start: foot_c[i], end: waist_c[i], radial_hint: radial });
			if gap > GAP_EPS {
				posts.push(PostSegment { start: waist_c[i], end: upper_c[i], radial_hint: radial });
			}
			posts.push(PostSegment { start: upper_c[i], end: ridge_c[i], radial_hint: radial });
		}

		let n = self.face_post_count;
		if n > 0 {
			for side in TrazaloidSide::all() {
				let outward = side.outward();
				let (la0, lb0) = face_bottom_pair(side, foot);
				let (la1, lb1) = face_bottom_pair(side, waist);
				push_face_posts(&mut posts, la0, lb0, la1, lb1, n, outward);
				let (ua0, ub0) = face_bottom_pair(side, upper_bot);
				let (ua1, ub1) = face_bottom_pair(side, ridge);
				push_face_posts(&mut posts, ua0, ub0, ua1, ub1, n, outward);
			}
		}

		posts
	}
}

/// Bottom-left / bottom-right of a face when viewed from outside (left = rail_a).
pub(super) fn face_bottom_pair(side: TrazaloidSide, rect: PlanRect) -> (Vec3, Vec3) {
	match side {
		TrazaloidSide::North => (rect.nw(), rect.ne()),
		TrazaloidSide::East => (rect.ne(), rect.se()),
		TrazaloidSide::South => (rect.se(), rect.sw()),
		TrazaloidSide::West => (rect.sw(), rect.nw()),
	}
}

fn push_face_posts(
	posts: &mut Vec<PostSegment>,
	a0: Vec3,
	b0: Vec3,
	a1: Vec3,
	b1: Vec3,
	count: u32,
	radial_hint: Vec3,
) {
	let denom = (count + 1) as f32;
	for i in 1..=count {
		let u = i as f32 / denom;
		let start = a0.lerp(b0, u);
		let end = a1.lerp(b1, u);
		posts.push(PostSegment { start, end, radial_hint });
	}
}
