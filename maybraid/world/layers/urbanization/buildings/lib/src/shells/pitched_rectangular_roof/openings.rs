//! Pitch and gable-end openings: passages / apertures clip faces and map contact.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

use crate::openings::{
	MappedOpening, MappedOpeningQuad, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings,
};

use super::geometry::RoofHalf;
use super::{PitchedRoof, PitchedRoofParams};

impl PitchedRoof {
	/// Authoring helper: thin AABB on half `half` centered in face UV.
	///
	/// `u` / `v` are normalized face coords (`u` along eave, `v` eave→ridge).
	/// `width` / `height` are meters in the face (along eave / along generator).
	pub fn pitch_opening(
		half: &RoofHalf,
		u: f32,
		v: f32,
		width: f32,
		height: f32,
		label: OpeningLabel,
	) -> Opening {
		let width = width.max(1e-3);
		let height = height.max(1e-3);
		let depth = 0.35;
		let center = half.pitch_point(u.clamp(0.05, 0.95), v.clamp(0.05, 0.95));
		let (eave_x, eave_z) = RoofHalf::eave_frame(half.eave_line);
		let outward = -eave_z;
		let half_w = width * 0.5;
		let half_h = height * 0.5;
		let eave_u = half.eave_line.0.lerp(half.eave_line.1, u.clamp(0.0, 1.0));
		let ridge_u = half.ridge_line.0.lerp(half.ridge_line.1, u.clamp(0.0, 1.0));
		let up = (ridge_u - eave_u).normalize_or_zero();
		let min = center - eave_x * half_w - up * half_h - outward * depth;
		let max = center + eave_x * half_w + up * half_h + outward * depth * 0.25;
		Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
	}

	/// Authoring helper: window / door AABB on the full gable end wall (both halves).
	///
	/// `end` is line endpoint `.0` / `.1`. `width` spans the gable base; `height` rises
	/// toward the ridge from the wall-plate line.
	pub fn gable_end_opening(
		halves: &[RoofHalf; 2],
		end: usize,
		width: f32,
		height: f32,
		label: OpeningLabel,
	) -> Opening {
		let (e_pos, e_neg, ridge) = gable_end_corners(halves, end);
		let width = width.max(1e-3);
		let height = height.max(1e-3);
		let depth = 0.4;
		let base_mid = (e_pos + e_neg) * 0.5;
		let across = (e_pos - e_neg).normalize_or_zero();
		let up = (ridge - base_mid).normalize_or_zero();
		let (eave_x, _) = RoofHalf::eave_frame(halves[0].eave_line);
		let outward = if end == 0 { -eave_x } else { eave_x };
		// Sit the opening above the wall plate, not flush with the peak.
		let center = base_mid + up * (0.35 + height * 0.5);
		let half_w = width * 0.5;
		let half_h = height * 0.5;
		let min = center - across * half_w - up * half_h - outward * depth;
		let max = center + across * half_w + up * half_h + outward * depth * 0.25;
		Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpeningFace {
	Pitch(usize),
	GableEnd(usize),
}

/// Resolved pitch clip (at most one per half).
pub(super) struct PitchOpeningMap {
	pub half: usize,
	pub clip: Vec<Vec3>,
	pub id: OpeningId,
	pub opening: Opening,
	pub mapped: MappedOpening,
}

/// Resolved gable-end clip (at most one per line end; applied to both halves).
pub(super) struct GableOpeningMap {
	pub end: usize,
	pub clip: Vec<Vec3>,
	pub id: OpeningId,
	pub opening: Opening,
	pub mapped: MappedOpening,
}

pub(super) struct ResolvedRoofOpenings {
	pub pitch: [Option<PitchOpeningMap>; 2],
	pub gable: [Option<GableOpeningMap>; 2],
}

impl PitchedRoofParams {
	/// Map passages / apertures onto pitch halves and drawn gable ends.
	///
	/// Each opening goes to the nearest face; largest extent wins per face slot.
	pub(super) fn resolve_roof_openings(&self) -> ResolvedRoofOpenings {
		let faces = available_faces(&self.halves);
		let mut best: Vec<(OpeningFace, f32, OpeningId, Opening)> = Vec::new();

		for (id, opening) in self.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Aperture) {
				continue;
			}
			let Some(face) = best_face_for_bounds(&opening.bounds, &self.halves, &faces) else {
				continue;
			};
			let score = face_extent_score(&opening.bounds, &self.halves, face);
			if let Some(slot) = best.iter_mut().find(|(f, ..)| *f == face) {
				if score > slot.1 {
					*slot = (face, score, id.clone(), opening.clone());
				}
			} else {
				best.push((face, score, id.clone(), opening.clone()));
			}
		}

		let mut pitch: [Option<PitchOpeningMap>; 2] = [None, None];
		let mut gable: [Option<GableOpeningMap>; 2] = [None, None];
		for (face, _, id, opening) in best {
			match face {
				OpeningFace::Pitch(half) => {
					let roof_half = &self.halves[half];
					let (face_width, face_height) = pitch_face_extents(roof_half);
					let (width, height) =
						opening_dims_on_pitch(&opening.bounds, roof_half, face_width, face_height);
					let (u, v) = opening_uv_on_pitch(&opening.bounds, roof_half);
					let clip = centered_pitch_clip(roof_half, u, v, width, height);
					let orientation = RoofHalf::outward_orientation(roof_half.eave_line);
					let mapped = mapped_from_outside_clip(&clip, orientation);
					pitch[half] = Some(PitchOpeningMap { half, clip, id, opening, mapped });
				}
				OpeningFace::GableEnd(end) => {
					let (e_pos, e_neg, ridge) = gable_end_corners(&self.halves, end);
					let (width, height) =
						opening_dims_on_gable(&opening.bounds, e_pos, e_neg, ridge);
					let clip = centered_gable_clip(e_pos, e_neg, ridge, width, height);
					let (eave_x, _) = RoofHalf::eave_frame(self.halves[0].eave_line);
					let outward = if end == 0 { -eave_x } else { eave_x };
					let orientation = Vec2::new(outward.x, outward.z);
					let mapped = mapped_from_outside_clip(&clip, orientation);
					gable[end] = Some(GableOpeningMap { end, clip, id, opening, mapped });
				}
			}
		}

		ResolvedRoofOpenings { pitch, gable }
	}
}

impl MapsOpenings for PitchedRoof {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}

fn available_faces(halves: &[RoofHalf; 2]) -> Vec<OpeningFace> {
	let mut faces = vec![OpeningFace::Pitch(0), OpeningFace::Pitch(1)];
	for end in 0..2 {
		let drawn = halves.iter().any(|h| {
			if end == 0 {
				h.draw_in_half_gable_end.0
			} else {
				h.draw_in_half_gable_end.1
			}
		});
		if drawn {
			faces.push(OpeningFace::GableEnd(end));
		}
	}
	faces
}

fn gable_end_corners(halves: &[RoofHalf; 2], end: usize) -> (Vec3, Vec3, Vec3) {
	let e_pos = RoofHalf::line_end(halves[0].eave_line, end);
	let e_neg = RoofHalf::line_end(halves[1].eave_line, end);
	let ridge = RoofHalf::line_end(halves[0].ridge_line, end);
	(e_pos, e_neg, ridge)
}

fn gable_end_centroid(halves: &[RoofHalf; 2], end: usize) -> Vec3 {
	let (e_pos, e_neg, ridge) = gable_end_corners(halves, end);
	(e_pos + e_neg + ridge) / 3.0
}

fn best_face_for_bounds(
	bounds: &Aabb3d,
	halves: &[RoofHalf; 2],
	faces: &[OpeningFace],
) -> Option<OpeningFace> {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let mut best: Option<(OpeningFace, f32)> = None;
	for &face in faces {
		let c = match face {
			OpeningFace::Pitch(h) => halves[h].pitch_centroid(),
			OpeningFace::GableEnd(end) => gable_end_centroid(halves, end),
		};
		let d = mid.distance_squared(c);
		let replace = match best {
			None => true,
			Some((_, prev)) => d < prev,
		};
		if replace {
			best = Some((face, d));
		}
	}
	best.map(|(f, _)| f)
}

fn face_extent_score(bounds: &Aabb3d, halves: &[RoofHalf; 2], face: OpeningFace) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	match face {
		OpeningFace::Pitch(h) => pitch_extent_score(bounds, &halves[h]),
		OpeningFace::GableEnd(_) => e.x.max(e.z).max(0.0) * e.y.max(0.0),
	}
}

fn pitch_extent_score(bounds: &Aabb3d, half: &RoofHalf) -> f32 {
	let e = Vec3::from(bounds.max - bounds.min);
	let (eave_x, _) = RoofHalf::eave_frame(half.eave_line);
	let along = (e.x * eave_x.x.abs() + e.z * eave_x.z.abs()).max(e.x.max(e.z) * 0.5);
	along.max(0.0) * e.y.max(0.0)
}

fn pitch_face_extents(half: &RoofHalf) -> (f32, f32) {
	let (e0, e1) = half.eave_line;
	let (r0, _) = half.ridge_line;
	let face_width = e0.distance(e1).max(1e-4);
	let face_height = e0.distance(r0).max(1e-4);
	(face_width, face_height)
}

fn opening_uv_on_pitch(bounds: &Aabb3d, half: &RoofHalf) -> (f32, f32) {
	let mid = Vec3::from((bounds.min + bounds.max) * 0.5);
	let (e0, e1) = half.eave_line;
	let (r0, r1) = half.ridge_line;
	let eave_dir = (e1 - e0).normalize_or_zero();
	let eave_len = e0.distance(e1).max(1e-4);
	let u = ((mid - e0).dot(eave_dir) / eave_len).clamp(0.05, 0.95);
	let eave_u = e0.lerp(e1, u);
	let ridge_u = r0.lerp(r1, u);
	let gen = ridge_u - eave_u;
	let gen_len = gen.length().max(1e-4);
	let v = ((mid - eave_u).dot(gen / gen_len) / gen_len).clamp(0.05, 0.95);
	(u, v)
}

fn opening_dims_on_pitch(
	bounds: &Aabb3d,
	half: &RoofHalf,
	face_width: f32,
	face_height: f32,
) -> (f32, f32) {
	let e = Vec3::from(bounds.max - bounds.min);
	let (eave_x, _) = RoofHalf::eave_frame(half.eave_line);
	let width = (e.x * eave_x.x.abs() + e.z * eave_x.z.abs())
		.max(e.x.max(e.z) * 0.5)
		.clamp(face_width * 0.05, face_width * 0.95);
	let height = e.y.clamp(face_height * 0.05, face_height * 0.95);
	(width, height)
}

fn opening_dims_on_gable(bounds: &Aabb3d, e_pos: Vec3, e_neg: Vec3, ridge: Vec3) -> (f32, f32) {
	let e = Vec3::from(bounds.max - bounds.min);
	let base = e_pos.distance(e_neg).max(1e-4);
	let rise = ((e_pos + e_neg) * 0.5).distance(ridge).max(1e-4);
	let width = e.x.max(e.z).clamp(base * 0.05, base * 0.95);
	let height = e.y.clamp(rise * 0.05, rise * 0.95);
	(width, height)
}

/// Centered rectangular clip on the pitch face (`[BL, BR, TR, TL]` looking in).
pub(crate) fn centered_pitch_clip(
	half: &RoofHalf,
	u: f32,
	v: f32,
	width: f32,
	height: f32,
) -> Vec<Vec3> {
	let (face_width, face_height) = pitch_face_extents(half);
	let width_frac = (width / face_width).clamp(0.05, 0.95);
	let height_frac = (height / face_height).clamp(0.05, 0.95);
	let u0 = (u - width_frac * 0.5).clamp(0.0, 1.0 - width_frac);
	let u1 = u0 + width_frac;
	let v0 = (v - height_frac * 0.5).clamp(0.0, 1.0 - height_frac);
	let v1 = v0 + height_frac;
	vec![
		half.pitch_point(u0, v0),
		half.pitch_point(u1, v0),
		half.pitch_point(u1, v1),
		half.pitch_point(u0, v1),
	]
}

/// Centered clip on the full gable triangle (`[BL, BR, TR, TL]` looking in).
pub(crate) fn centered_gable_clip(
	e_pos: Vec3,
	e_neg: Vec3,
	ridge: Vec3,
	width: f32,
	height: f32,
) -> Vec<Vec3> {
	let base_mid = (e_pos + e_neg) * 0.5;
	let across = (e_pos - e_neg).normalize_or_zero();
	let up = (ridge - base_mid).normalize_or_zero();
	let base_span = e_pos.distance(e_neg).max(1e-4);
	let rise = base_mid.distance(ridge).max(1e-4);
	let half_w = (width * 0.5).min(base_span * 0.45);
	let half_h = (height * 0.5).min(rise * 0.4);
	let center = base_mid + up * (0.35 + half_h);
	vec![
		center - across * half_w - up * half_h,
		center + across * half_w - up * half_h,
		center + across * half_w + up * half_h,
		center - across * half_w + up * half_h,
	]
}

/// `clip` is `[BL, BR, TR, TL]` looking **in** from outside; map wants looking **out**.
fn mapped_from_outside_clip(clip: &[Vec3], orientation: Vec2) -> MappedOpening {
	debug_assert!(clip.len() >= 4);
	MappedOpening::new(MappedOpeningQuad::new(clip[1], clip[0], clip[2], clip[3]), orientation)
}
