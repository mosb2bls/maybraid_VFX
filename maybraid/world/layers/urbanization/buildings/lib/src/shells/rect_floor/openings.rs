//! Wall opening resolution: positioned RectInset per cardinal side.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};

use crate::openings::{MappedOpenings, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::RectInset;
use crate::shells::ortho::{
	best_side_for_bounds, face_extent_score, ntube_face_opening, OrthoSide, PlanRect,
};

use super::{RectFloor, RectFloorParams};

/// Cardinal side of a [`RectFloor`] (alias of [`OrthoSide`]).
pub type RectFloorSide = OrthoSide;

impl RectFloor {
	/// Authoring helper: thin passage AABB on a footprint face.
	pub fn side_passage_opening(
		side: RectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
	) -> Opening {
		side_opening(side, center_xz, footprint, width, height, OpeningLabel::Passage)
	}

	/// Authoring helper: thin aperture AABB on a footprint face.
	pub fn side_aperture_opening(
		side: RectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
		sill: f32,
	) -> Opening {
		let mut o = side_opening(side, center_xz, footprint, width, height, OpeningLabel::Aperture);
		let min = Vec3::from(o.bounds.min) + Vec3::Y * sill.max(0.0);
		let max = Vec3::from(o.bounds.max) + Vec3::Y * sill.max(0.0);
		o.bounds = Aabb3d::from_min_max(min, max);
		o
	}
}

fn side_opening(
	side: RectFloorSide,
	center_xz: Vec3,
	footprint: Vec2,
	width: f32,
	height: f32,
	label: OpeningLabel,
) -> Opening {
	let half_x = footprint.x * 0.5;
	let half_z = footprint.y * 0.5;
	let width = width.max(1e-3);
	let height = height.max(1e-3);
	let depth = 0.4;
	let cx = center_xz.x;
	let cy = center_xz.y;
	let cz = center_xz.z;
	let (min, max) = match side {
		OrthoSide::North => (
			Vec3::new(cx - width * 0.5, cy, cz + half_z - depth),
			Vec3::new(cx + width * 0.5, cy + height, cz + half_z + depth * 0.25),
		),
		OrthoSide::South => (
			Vec3::new(cx - width * 0.5, cy, cz - half_z - depth * 0.25),
			Vec3::new(cx + width * 0.5, cy + height, cz - half_z + depth),
		),
		OrthoSide::East => (
			Vec3::new(cx + half_x - depth, cy, cz - width * 0.5),
			Vec3::new(cx + half_x + depth * 0.25, cy + height, cz + width * 0.5),
		),
		OrthoSide::West => (
			Vec3::new(cx - half_x - depth * 0.25, cy, cz - width * 0.5),
			Vec3::new(cx - half_x + depth, cy + height, cz + width * 0.5),
		),
	};
	Opening::new(Aabb3d::from_min_max(min.min(max), min.max(max)), label)
}

impl RectFloorParams {
	/// Per-side insets + retained/mapped openings.
	pub(super) fn resolve_wall_openings(
		&self,
		plan: PlanRect,
	) -> ([Option<RectInset>; 4], Openings, MappedOpenings) {
		let y0 = plan.y;
		let y1 = plan.y + self.storey_height;
		let sw = Vec3::new(plan.sw().x, y0, plan.sw().z);
		let se = Vec3::new(plan.se().x, y0, plan.se().z);
		let ne = Vec3::new(plan.ne().x, y0, plan.ne().z);
		let nw = Vec3::new(plan.nw().x, y0, plan.nw().z);
		let sw1 = Vec3::new(sw.x, y1, sw.z);
		let se1 = Vec3::new(se.x, y1, se.z);
		let ne1 = Vec3::new(ne.x, y1, ne.z);
		let nw1 = Vec3::new(nw.x, y1, nw.z);
		let corners = [
			(sw, se, sw1), // South
			(se, ne, se1), // East
			(ne, nw, ne1), // North
			(nw, sw, nw1), // West
		];

		let mut best: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];
		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let side = best_side_for_bounds(&opening.bounds, plan);
			let along_x = matches!(side, OrthoSide::North | OrthoSide::South);
			let score = face_extent_score(&opening.bounds, along_x);
			let idx = side.face_index();
			let replace = match &best[idx] {
				None => true,
				Some((prev, ..)) => score > *prev,
			};
			if replace {
				best[idx] = Some((score, id.clone(), opening.clone()));
			}
		}

		let thickness = self.joint_thickness.max(1e-4);
		let mut insets = [None, None, None, None];
		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		for side in OrthoSide::all() {
			let idx = side.face_index();
			let Some((_, id, opening)) = best[idx].take() else {
				continue;
			};
			let (a0, b0, a1) = corners[idx];
			let Some(face) = ntube_face_opening(a0, b0, a1, &opening.bounds, thickness) else {
				continue;
			};
			insets[idx] = Some(face.inset);
			mapped.insert(id.clone(), face.mapped);
			openings.insert(id, opening);
		}
		(insets, openings, mapped)
	}
}

impl MapsOpenings for RectFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&crate::openings::MappedOpening> {
		self.mapped.get(id)
	}
}
