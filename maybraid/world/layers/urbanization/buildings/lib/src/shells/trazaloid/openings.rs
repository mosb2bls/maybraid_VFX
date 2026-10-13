//! Passage → centered lower-band door clips; apertures are not mapped.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings,
};

use super::geometry::{face_bottom_pair, PlanRect, TrazaloidSide};
use super::{Trazaloid, TrazaloidParams};

impl Trazaloid {
	/// Authoring helper: thin centered passage AABB on a footprint face.
	pub fn side_passage_opening(
		side: TrazaloidSide,
		footprint: Vec2,
		width: f32,
		height: f32,
	) -> Opening {
		let half_x = footprint.x * 0.5;
		let half_z = footprint.y * 0.5;
		let width = width.max(1e-3);
		let height = height.max(1e-3);
		let depth = 0.4;
		let (min, max) = match side {
			TrazaloidSide::North => (
				Vec3::new(-width * 0.5, 0.0, half_z - depth),
				Vec3::new(width * 0.5, height, half_z + depth * 0.25),
			),
			TrazaloidSide::South => (
				Vec3::new(-width * 0.5, 0.0, -half_z - depth * 0.25),
				Vec3::new(width * 0.5, height, -half_z + depth),
			),
			TrazaloidSide::East => (
				Vec3::new(half_x - depth, 0.0, -width * 0.5),
				Vec3::new(half_x + depth * 0.25, height, width * 0.5),
			),
			TrazaloidSide::West => (
				Vec3::new(-half_x - depth * 0.25, 0.0, -width * 0.5),
				Vec3::new(-half_x + depth, height, width * 0.5),
			),
		};
		Opening::passage(Aabb3d::from_min_max(min.min(max), min.max(max)))
	}
}

/// Per-side door clip + mapped opening after passage resolution.
pub(super) struct SidePassageMap {
	pub clip: Vec<Vec3>,
	pub id: OpeningId,
	pub opening: Opening,
	pub mapped: MappedOpening,
}

impl TrazaloidParams {
	/// Map passages onto lower-band sides: largest face-aligned extent wins per side.
	///
	/// Apertures are ignored (the waist band gap is the window). Width / height of
	/// the centered door come from the winning opening AABB.
	pub(super) fn resolve_side_passages(
		&self,
		foot: PlanRect,
		waist: PlanRect,
	) -> [Option<SidePassageMap>; 4] {
		let face_height = (waist.y - foot.y).max(1e-4);
		// Best passage per side: (score, id, opening).
		let mut best: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];

		for (id, opening) in self.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let Some(side) = best_side_for_bounds(&opening.bounds, foot) else {
				continue;
			};
			let score = passage_extent_score(&opening.bounds, side);
			let idx = side as usize;
			let replace = match &best[idx] {
				None => true,
				Some((prev, ..)) => score > *prev,
			};
			if replace {
				best[idx] = Some((score, id.clone(), opening.clone()));
			}
		}

		let mut out: [Option<SidePassageMap>; 4] = [None, None, None, None];
		for side in TrazaloidSide::all() {
			let idx = side as usize;
			let Some((_, id, opening)) = best[idx].take() else {
				continue;
			};
			let (a0, b0) = face_bottom_pair(side, foot);
			let (a1, b1) = face_bottom_pair(side, waist);
			let face_width = a0.distance(b0).max(1e-4);
			let (width, height) =
				door_dims_from_bounds(&opening.bounds, side, face_width, face_height);
			let clip = ground_door_clip(a0, b0, a1, b1, width, height);
			let mapped = mapped_from_outside_clip(&clip, side.orientation());
			out[idx] = Some(SidePassageMap { clip, id, opening, mapped });
		}
		out
	}
}

impl MapsOpenings for Trazaloid {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}

fn passage_extent_score(bounds: &Aabb3d, side: TrazaloidSide) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	let width = match side {
		TrazaloidSide::North | TrazaloidSide::South => e.x,
		TrazaloidSide::East | TrazaloidSide::West => e.z,
	};
	width.max(0.0) * e.y.max(0.0)
}

fn door_dims_from_bounds(
	bounds: &Aabb3d,
	side: TrazaloidSide,
	face_width: f32,
	face_height: f32,
) -> (f32, f32) {
	let e = Vec3::from(bounds.max - bounds.min);
	let width = match side {
		TrazaloidSide::North | TrazaloidSide::South => e.x,
		TrazaloidSide::East | TrazaloidSide::West => e.z,
	}
	.clamp(face_width * 0.05, face_width * 0.95);
	let height = e.y.clamp(face_height * 0.05, face_height * 0.95);
	(width, height)
}

fn best_side_for_bounds(bounds: &Aabb3d, foot: PlanRect) -> Option<TrazaloidSide> {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let candidates = [
		(TrazaloidSide::North, Vec3::new(foot.cx, mid.y, foot.cz + foot.half_z)),
		(TrazaloidSide::South, Vec3::new(foot.cx, mid.y, foot.cz - foot.half_z)),
		(TrazaloidSide::East, Vec3::new(foot.cx + foot.half_x, mid.y, foot.cz)),
		(TrazaloidSide::West, Vec3::new(foot.cx - foot.half_x, mid.y, foot.cz)),
	];
	candidates
		.into_iter()
		.min_by(|(_, a), (_, b)| {
			mid.distance_squared(*a)
				.partial_cmp(&mid.distance_squared(*b))
				.unwrap_or(std::cmp::Ordering::Equal)
		})
		.map(|(side, _)| side)
}

/// `clip` is `[BL, BR, TR, TL]` looking **in** from outside; map wants looking **out**.
fn mapped_from_outside_clip(clip: &[Vec3], orientation: Vec2) -> MappedOpening {
	debug_assert!(clip.len() >= 4);
	MappedOpening::new(MappedOpeningQuad::new(clip[1], clip[0], clip[2], clip[3]), orientation)
}

/// Centered door opening on the face, flush with the ground, sized in meters.
pub(crate) fn ground_door_clip(
	a0: Vec3,
	b0: Vec3,
	a1: Vec3,
	b1: Vec3,
	width: f32,
	height: f32,
) -> Vec<Vec3> {
	let face_width = a0.distance(b0).max(1e-4);
	let face_height = a0.distance(a1).max(1e-4);
	let width_frac = (width / face_width).clamp(0.05, 0.95);
	let height_frac = (height / face_height).clamp(0.05, 0.95);
	let u0 = (1.0 - width_frac) * 0.5;
	let u1 = u0 + width_frac;
	let v0 = 0.0;
	let v1 = height_frac;
	let p = |u: f32, v: f32| {
		let bottom = a0.lerp(b0, u);
		let top = a1.lerp(b1, u);
		bottom.lerp(top, v)
	};
	vec![p(u0, v0), p(u1, v0), p(u1, v1), p(u0, v1)]
}
