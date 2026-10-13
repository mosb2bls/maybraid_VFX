//! Annulus floor / ceiling via [`ApproximatedCircle`] (`clip = inner_radius`).
//!
//! v1: `cuts_slab` openings that erase enough of the annulus omit the whole
//! piece. Positioned polygonal bites are not supported by ApproximatedCircle.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use crate::openings::Openings;
use crate::paneling::approximated_circle::ApproximatedCircle;
use crate::shells::ortho::SLAB_Y_HALF;

use super::{CircRingFloorParams, CircRingFloorSlab};

const EPS: f32 = 1e-4;

impl CircRingFloorParams {
	pub(super) fn resolve_slab(
		&self,
		slab: CircRingFloorSlab,
		center: Vec3,
	) -> Option<ApproximatedCircle> {
		match slab {
			CircRingFloorSlab::None => None,
			CircRingFloorSlab::Solid => {
				if annulus_erased(center, self.outer_radius, self.inner_radius, &self.openings) {
					None
				} else {
					Some(ApproximatedCircle::horizontal(
						self.style,
						center,
						self.outer_radius,
						self.segments,
						Some(self.inner_radius),
					))
				}
			}
		}
	}
}

fn annulus_erased(center: Vec3, outer_r: f32, inner_r: f32, openings: &Openings) -> bool {
	let slab_aabb = Aabb3d::from_min_max(
		Vec3::new(center.x - outer_r, center.y - SLAB_Y_HALF, center.z - outer_r),
		Vec3::new(center.x + outer_r, center.y + SLAB_Y_HALF, center.z + outer_r),
	);
	let band = (outer_r - inner_r).max(EPS);
	for (_id, opening) in openings.iter() {
		if !opening.label.cuts_slab() {
			continue;
		}
		let Some(inter) = aabb_intersection(&opening.bounds, &slab_aabb) else {
			continue;
		};
		let extent = Vec3::from(inter.max - inter.min);
		let scale = extent.x.max(extent.z);
		// Full-ish diameter bite, or a bite that spans the radial band and a
		// large chord of the outer ring.
		if scale + EPS >= outer_r * 1.5 || (scale >= outer_r && scale >= band * 0.9) {
			return true;
		}
		// Radial coverage heuristic: opening reaches from near-inner to near-outer.
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		let corners = [
			Vec3::new(omin.x, center.y, omin.z),
			Vec3::new(omax.x, center.y, omin.z),
			Vec3::new(omin.x, center.y, omax.z),
			Vec3::new(omax.x, center.y, omax.z),
		];
		let mut r_min = f32::INFINITY;
		let mut r_max = 0.0f32;
		for p in corners {
			let r = (p - center).length();
			r_min = r_min.min(r);
			r_max = r_max.max(r);
		}
		let cover_lo = r_min.max(inner_r);
		let cover_hi = r_max.min(outer_r);
		if cover_hi - cover_lo >= band * 0.9 && scale >= outer_r * 0.75 {
			return true;
		}
	}
	false
}

fn aabb_intersection(a: &Aabb3d, b: &Aabb3d) -> Option<Aabb3d> {
	let min = Vec3::from(a.min).max(Vec3::from(b.min));
	let max = Vec3::from(a.max).min(Vec3::from(b.max));
	if min.x < max.x - EPS && min.y < max.y - EPS && min.z < max.z - EPS {
		Some(Aabb3d::from_min_max(min, max))
	} else {
		None
	}
}
