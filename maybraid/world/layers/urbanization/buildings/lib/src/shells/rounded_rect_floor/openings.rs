//! Wall opening resolution: rectangle straights + clipped quarter-arc corners.

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::partitions::PartitionStyle;
use std::f32::consts::FRAC_PI_2;

use crate::arcs::ClippedArcSweep;
use crate::openings::{
	MappedOpening, MappedOpeningQuad, MappedOpenings, MapsOpenings, Opening, OpeningId, Openings,
};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::shells::ortho::{edge_score_for_bounds, standing_face_opening, OrthoSide, EPS};

use super::geometry::{RoundedRectCorner, RoundedRectGeom};
use super::{RoundedRectFloor, RoundedRectFloorParams};

/// Cardinal straight of a [`RoundedRectFloor`].
pub type RoundedRectFloorSide = OrthoSide;

impl RoundedRectFloor {
	pub fn side_passage_opening(
		side: RoundedRectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_passage_opening(
			side, center_xz, footprint, width, height,
		)
	}

	pub fn side_aperture_opening(
		side: RoundedRectFloorSide,
		center_xz: Vec3,
		footprint: Vec2,
		width: f32,
		height: f32,
		sill: f32,
	) -> Opening {
		crate::shells::rect_floor::RectFloor::side_aperture_opening(
			side, center_xz, footprint, width, height, sill,
		)
	}
}

impl RoundedRectFloorParams {
	pub(super) fn resolve_walls(
		&self,
		geom: &RoundedRectGeom,
	) -> ([ClippedRectangularStrip; 4], [Option<ClippedArcSweep>; 4], Openings, MappedOpenings) {
		let thickness = self.joint_thickness.max(1e-4);
		let mut best_straight: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];
		let mut best_corner: [Option<(f32, OpeningId, Opening)>; 4] = [None, None, None, None];

		for (id, opening) in self.openings.iter() {
			if !opening.label.is_connectable() {
				continue;
			}
			let mut best_kind = None::<(bool, usize, f32, f32)>; // (is_corner, idx, dist, score)
			for (i, edge) in geom.straights.iter().enumerate() {
				if edge.length() < EPS {
					continue;
				}
				let (dist, score) = edge_score_for_bounds(&opening.bounds, *edge);
				let replace = match best_kind {
					None => true,
					Some((_, _, prev_d, prev_s)) => {
						dist < prev_d - 1e-4 || (dist <= prev_d + 1e-4 && score > prev_s)
					}
				};
				if replace {
					best_kind = Some((false, i, dist, score));
				}
			}
			if geom.radius > EPS {
				for corner in RoundedRectCorner::all() {
					let i = corner.index();
					let c = corner.center(geom.plan, geom.radius);
					let mid = c + Vec3::Y * (geom.height * 0.5);
					let opening_mid = Vec3::from((opening.bounds.min + opening.bounds.max) * 0.5);
					let dist = opening_mid.distance_squared(mid);
					let score = geom.radius * geom.height * 0.1;
					let replace = match best_kind {
						None => true,
						Some((_, _, prev_d, prev_s)) => {
							dist + 0.25 < prev_d || (dist <= prev_d + 1e-4 && score > prev_s)
						}
					};
					if replace {
						best_kind = Some((true, i, dist, score));
					}
				}
			}

			let Some((is_corner, idx, _, score)) = best_kind else {
				continue;
			};
			if is_corner {
				let replace = match &best_corner[idx] {
					None => true,
					Some((prev, ..)) => score > *prev,
				};
				if replace {
					best_corner[idx] = Some((score, id.clone(), opening.clone()));
				}
			} else {
				let replace = match &best_straight[idx] {
					None => true,
					Some((prev, ..)) => score > *prev,
				};
				if replace {
					best_straight[idx] = Some((score, id.clone(), opening.clone()));
				}
			}
		}

		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut straight_insets = [None, None, None, None];
		for side in OrthoSide::all() {
			let idx = side.face_index();
			let Some((_, id, opening)) = best_straight[idx].take() else {
				continue;
			};
			let edge = geom.straights[idx];
			if edge.length() < EPS {
				continue;
			}
			let Some(face) = standing_face_opening(edge, &opening.bounds, thickness) else {
				continue;
			};
			straight_insets[idx] = Some(face.inset);
			mapped.insert(id.clone(), face.mapped);
			openings.insert(id, opening);
		}

		let mut corner_clips: [Option<(f32, f32)>; 4] = [None, None, None, None];
		for corner in RoundedRectCorner::all() {
			let idx = corner.index();
			let Some((_, id, opening)) = best_corner[idx].take() else {
				continue;
			};
			let Some((t0, t1, mapped_o)) = corner_clip_t(geom, corner, &opening.bounds) else {
				continue;
			};
			corner_clips[idx] = Some((t0, t1));
			mapped.insert(id.clone(), mapped_o);
			openings.insert(id, opening);
		}

		let straights = OrthoSide::all().map(|side| {
			let idx = side.face_index();
			let edge = geom.straights[idx];
			if edge.length() < EPS {
				return ClippedRectangularStrip::new(self.style);
			}
			ClippedRectangularStrip::from_nodes(
				self.style,
				[
					RectangularStripNode::new(edge.start, edge.height, thickness, 0.0),
					RectangularStripNode::new(edge.end, edge.height, thickness, 0.0),
				],
				[straight_insets[idx]],
			)
		});

		let corners = RoundedRectCorner::all().map(|corner| {
			let idx = corner.index();
			if geom.radius < EPS {
				return None;
			}
			let c = corner.center(geom.plan, geom.radius);
			let clips = corner_clips[idx].into_iter();
			Some(ClippedArcSweep::new(
				c,
				geom.radius,
				geom.height,
				90.0,
				corner.start_yaw(),
				PartitionStyle::RoughStonework,
				clips,
			))
		});

		(straights, corners, openings, mapped)
	}
}

/// Map an opening AABB onto the quarter-arc as a normalized \(t\) clip + contact quad.
fn corner_clip_t(
	geom: &RoundedRectGeom,
	corner: RoundedRectCorner,
	bounds: &Aabb3d,
) -> Option<(f32, f32, MappedOpening)> {
	let c = corner.center(geom.plan, geom.radius);
	let start = corner.start_angle();
	let imin = Vec3::from(bounds.min);
	let imax = Vec3::from(bounds.max);
	let y0 = geom.plan.y;
	let y1 = y0 + geom.height;
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
	let mut t_lo = 1.0f32;
	let mut t_hi = 0.0f32;
	for p in corners_xz {
		let d = p - c_xz;
		if d.length_squared() < 1e-8 {
			continue;
		}
		let ang = d.y.atan2(d.x);
		let mut delta = ang - start;
		while delta < 0.0 {
			delta += std::f32::consts::TAU;
		}
		while delta > std::f32::consts::TAU {
			delta -= std::f32::consts::TAU;
		}
		if delta > FRAC_PI_2 + 0.35 {
			// Far outside this quarter — skip.
			continue;
		}
		let t = (delta / FRAC_PI_2).clamp(0.0, 1.0);
		t_lo = t_lo.min(t);
		t_hi = t_hi.max(t);
	}
	if t_hi - t_lo < 0.05 {
		// Degenerate: expand around midpoint of AABB.
		let mid = Vec2::new(0.5 * (imin.x + imax.x), 0.5 * (imin.z + imax.z));
		let d = mid - c_xz;
		let ang = d.y.atan2(d.x);
		let mut delta = ang - start;
		while delta < 0.0 {
			delta += std::f32::consts::TAU;
		}
		let t = (delta / FRAC_PI_2).clamp(0.05, 0.95);
		t_lo = (t - 0.08).clamp(0.0, 1.0);
		t_hi = (t + 0.08).clamp(0.0, 1.0);
	}
	if t_hi - t_lo < EPS {
		return None;
	}

	let ang0 = start + t_lo * FRAC_PI_2;
	let ang1 = start + t_hi * FRAC_PI_2;
	let r = geom.radius;
	let p0 = Vec3::new(c.x + ang0.cos() * r, hy0, c.z + ang0.sin() * r);
	let p1 = Vec3::new(c.x + ang1.cos() * r, hy0, c.z + ang1.sin() * r);
	let p2 = Vec3::new(c.x + ang0.cos() * r, hy1, c.z + ang0.sin() * r);
	let p3 = Vec3::new(c.x + ang1.cos() * r, hy1, c.z + ang1.sin() * r);
	let mapped = MappedOpening::new(MappedOpeningQuad::new(p1, p0, p3, p2), corner.outward());
	Some((t_lo, t_hi, mapped))
}

impl MapsOpenings for RoundedRectFloor {
	fn openings(&self) -> &Openings {
		&self.openings
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		self.mapped.get(id)
	}
}
