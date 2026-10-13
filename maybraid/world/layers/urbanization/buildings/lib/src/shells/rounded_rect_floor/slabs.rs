//! Floor / ceiling: inset core + outer edge strips + quarter-disk fans.

use bevy_math::Vec3;
use building_components::panels::PanelStyle;
use std::f32::consts::FRAC_PI_2;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::{PanelComplex, PanelPoint};
use crate::paneling::RectInset;
use crate::shells::ortho::{merge_slab_insets, PlanRect, EPS};

use super::geometry::{core_plan, RoundedRectCorner};
use super::{RoundedRectFloorParams, RoundedRectFloorSlab, RoundedSlabPiece};

/// Core + four edge strips (against the outer straights) + quarter fans.
pub(super) struct SlabParts {
	pub core: Option<RoundedSlabPiece>,
	pub edges: Vec<RoundedSlabPiece>,
	pub quarters: Vec<PanelComplex>,
}

impl RoundedRectFloorParams {
	pub(super) fn resolve_slab_parts(
		&self,
		slab: RoundedRectFloorSlab,
		plan: PlanRect,
		radius: f32,
	) -> SlabParts {
		match slab {
			RoundedRectFloorSlab::None => {
				SlabParts { core: None, edges: Vec::new(), quarters: Vec::new() }
			}
			RoundedRectFloorSlab::Solid => {
				let cutting: Vec<_> = self
					.openings
					.iter()
					.filter_map(|(_id, o)| o.label.cuts_slab().then_some(o.bounds))
					.collect();
				let core = resolve_piece(
					self.style,
					core_plan(plan, radius),
					&cutting,
					self.joint_thickness,
				);

				let mut edges = Vec::new();
				if radius > EPS {
					for edge_plan in edge_strip_plans(plan, radius) {
						if let Some(piece) =
							resolve_piece(self.style, edge_plan, &cutting, self.joint_thickness)
						{
							edges.push(piece);
						}
					}
				}

				let mut quarters = Vec::new();
				if radius > EPS {
					for corner in RoundedRectCorner::all() {
						if let Some(q) = quarter_disk(
							self.style,
							plan,
							radius,
							self.corner_segments.max(1),
							corner,
							self.joint_thickness,
							&self.openings,
						) {
							quarters.push(q);
						}
					}
				}
				SlabParts { core, edges, quarters }
			}
		}
	}
}

/// Four axis-aligned strips between the inset core and the outer straight edges.
///
/// Each strip butts the quarter-disk corners at its ends and runs flush with the
/// outer footprint on the straight side.
fn edge_strip_plans(plan: PlanRect, radius: f32) -> [PlanRect; 4] {
	let r = radius.max(0.0);
	let cx = plan.center.x;
	let cz = plan.center.z;
	let hx = plan.half_x;
	let hz = plan.half_z;
	let y = plan.y;
	let mid_w = (plan.full_x() - 2.0 * r).max(EPS);
	let mid_d = (plan.full_z() - 2.0 * r).max(EPS);
	[
		// South: outer −Z edge, between SW/SE quarter disks.
		PlanRect::new(Vec3::new(cx, y, cz - hz + r * 0.5), mid_w, r),
		// East: outer +X edge, between SE/NE quarter disks.
		PlanRect::new(Vec3::new(cx + hx - r * 0.5, y, cz), r, mid_d),
		// North: outer +Z edge.
		PlanRect::new(Vec3::new(cx, y, cz + hz - r * 0.5), mid_w, r),
		// West: outer −X edge.
		PlanRect::new(Vec3::new(cx - hx + r * 0.5, y, cz), r, mid_d),
	]
}

fn resolve_piece(
	style: PanelStyle,
	plan: PlanRect,
	cutting: &[bevy_math::bounding::Aabb3d],
	thickness: f32,
) -> Option<RoundedSlabPiece> {
	match merge_slab_insets(plan, cutting.iter().copied()) {
		None => Some(RoundedSlabPiece::Solid(solid_rect(style, plan, thickness))),
		Some(None) => None,
		Some(Some(inset)) => {
			Some(RoundedSlabPiece::Clipped(clipped_rect(style, plan, thickness, inset)))
		}
	}
}

fn corners(plan: PlanRect, thickness: f32) -> [PanelPoint; 4] {
	let t = thickness.max(1e-4);
	[
		PanelPoint::new(plan.sw(), t),
		PanelPoint::new(plan.se(), t),
		PanelPoint::new(plan.nw(), t),
		PanelPoint::new(plan.ne(), t),
	]
}

fn solid_rect(style: PanelStyle, plan: PlanRect, thickness: f32) -> FittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	FittedRectangle::new(style, sw, se, nw, ne)
}

fn clipped_rect(
	style: PanelStyle,
	plan: PlanRect,
	thickness: f32,
	inset: RectInset,
) -> ClippedFittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	ClippedFittedRectangle::new(style, sw, se, nw, ne, inset)
}

/// Quarter disk at a corner. Omitted entirely when a cuts_slab opening covers its AABB.
fn quarter_disk(
	style: PanelStyle,
	plan: PlanRect,
	radius: f32,
	segments: u32,
	corner: RoundedRectCorner,
	thickness: f32,
	openings: &Openings,
) -> Option<PanelComplex> {
	let c = corner.center(plan, radius);
	let start = corner.start_angle();
	let n = segments.max(1);
	let mut amin = c;
	let mut amax = c;
	for i in 0..=n {
		let t = i as f32 / n as f32;
		let ang = start + t * FRAC_PI_2;
		let p = Vec3::new(c.x + ang.cos() * radius, plan.y, c.z + ang.sin() * radius);
		amin = amin.min(p);
		amax = amax.max(p);
	}
	let qplan = PlanRect::new(
		Vec3::new(0.5 * (amin.x + amax.x), plan.y, 0.5 * (amin.z + amax.z)),
		(amax.x - amin.x).max(EPS),
		(amax.z - amin.z).max(EPS),
	);
	for (_id, o) in openings.iter() {
		if !o.label.cuts_slab() {
			continue;
		}
		if let Some(None) = merge_slab_insets(qplan, std::iter::once(o.bounds)) {
			return None;
		}
	}

	let t = thickness.max(1e-4);
	let mut complex = PanelComplex::new(style);
	let center_id = complex.insert_point_thick(c, t);
	let mut ring = Vec::with_capacity(n as usize + 1);
	for i in 0..=n {
		let frac = i as f32 / n as f32;
		let ang = start + frac * FRAC_PI_2;
		let p = Vec3::new(c.x + ang.cos() * radius, plan.y, c.z + ang.sin() * radius);
		ring.push(complex.insert_point_thick(p, t));
	}
	for w in ring.windows(2) {
		complex.add_triangle(center_id, w[0], w[1]);
	}
	Some(complex)
}
