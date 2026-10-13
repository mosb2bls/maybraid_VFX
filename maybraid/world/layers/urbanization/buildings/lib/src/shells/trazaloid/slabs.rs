//! Layer 2: footprint / ridge slabs cut by non-passage, non-aperture openings.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::panels::PanelStyle;

use crate::openings::Openings;
use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::panel_complex::PanelComplexJointPolicy;

use super::geometry::{PlanRect, EXTENT_EPS};
use super::{TrazaloidParams, TrazaloidSlab};

const EPS: f32 = 1e-4;
/// Thin slab volume half-height for intersection tests.
const SLAB_Y_HALF: f32 = 0.2;

impl TrazaloidParams {
	pub(super) fn resolve_floor_slab(
		&self,
		style: PanelStyle,
		policy: PanelComplexJointPolicy,
		foot: PlanRect,
	) -> Option<ClippedRuledStrip> {
		resolve_horizontal_slab(style, policy, foot, self.floor, &self.openings)
	}

	pub(super) fn resolve_ceiling_slab(
		&self,
		style: PanelStyle,
		policy: PanelComplexJointPolicy,
		ridge: PlanRect,
	) -> Option<ClippedRuledStrip> {
		resolve_horizontal_slab(style, policy, ridge, self.ceiling, &self.openings)
	}
}

fn resolve_horizontal_slab(
	style: PanelStyle,
	policy: PanelComplexJointPolicy,
	rect: PlanRect,
	slab: TrazaloidSlab,
	openings: &Openings,
) -> Option<ClippedRuledStrip> {
	match slab {
		TrazaloidSlab::None => None,
		TrazaloidSlab::Solid => {
			let max_side = rect.full_x().min(rect.full_z());
			let slab_aabb = slab_volume_aabb(rect);
			let mut hole_side: Option<f32> = None;
			for (_id, opening) in openings.iter() {
				if !opening.label.cuts_slab() {
					continue;
				}
				if !aabb3d_intersects(&opening.bounds, &slab_aabb) {
					continue;
				}
				let Some(inter) = aabb_intersection(&opening.bounds, &slab_aabb) else {
					continue;
				};
				let extent = Vec3::from(inter.max - inter.min);
				let scale = extent.x.max(extent.z);
				if scale + EPS >= max_side {
					return None; // remove entire slab
				}
				hole_side = Some(hole_side.map_or(scale, |s| s.max(scale)));
			}
			let clip = hole_side.map(|side| {
				let max_half = (rect.half_x.min(rect.half_z) - EXTENT_EPS).max(EXTENT_EPS);
				let half = (side * 0.5).clamp(EXTENT_EPS, max_half);
				centered_square_clip(rect, half * 2.0)
			});
			Some(
				ClippedRuledStrip::from_lines(
					style,
					[rect.sw(), rect.se()],
					[rect.nw(), rect.ne()],
					[clip],
				)
				.with_joint_policy(policy),
			)
		}
	}
}

fn centered_square_clip(rect: PlanRect, size: f32) -> Vec<Vec3> {
	let max_half = (rect.half_x.min(rect.half_z) - EXTENT_EPS).max(EXTENT_EPS);
	let half = (size * 0.5).clamp(EXTENT_EPS, max_half);
	vec![
		Vec3::new(rect.cx - half, rect.y, rect.cz - half),
		Vec3::new(rect.cx + half, rect.y, rect.cz - half),
		Vec3::new(rect.cx + half, rect.y, rect.cz + half),
		Vec3::new(rect.cx - half, rect.y, rect.cz + half),
	]
}

fn slab_volume_aabb(rect: PlanRect) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(rect.cx - rect.half_x, rect.y - SLAB_Y_HALF, rect.cz - rect.half_z),
		Vec3::new(rect.cx + rect.half_x, rect.y + SLAB_Y_HALF, rect.cz + rect.half_z),
	)
}

fn aabb_intersection(a: &Aabb3d, b: &Aabb3d) -> Option<Aabb3d> {
	if !aabb3d_intersects(a, b) {
		return None;
	}
	let min = Vec3::from(a.min).max(Vec3::from(b.min));
	let max = Vec3::from(a.max).min(Vec3::from(b.max));
	Some(Aabb3d::from_min_max(min, max))
}

fn aabb3d_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - EPS
		&& a.max.x > b.min.x + EPS
		&& a.min.y < b.max.y - EPS
		&& a.max.y > b.min.y + EPS
		&& a.min.z < b.max.z - EPS
		&& a.max.z > b.min.z + EPS
}
