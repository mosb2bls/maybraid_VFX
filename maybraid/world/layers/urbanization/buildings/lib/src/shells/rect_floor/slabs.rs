//! Layer 2: positioned floor / ceiling cuts from cuts_slab openings.

use building_components::panels::PanelStyle;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::PanelPoint;
use crate::paneling::RectInset;
use crate::shells::ortho::{merge_slab_insets, PlanRect};

use super::{RectFloorParams, RectFloorSlab, RectFloorSlabGeom};

impl RectFloorParams {
	pub(super) fn resolve_slab(
		&self,
		slab: RectFloorSlab,
		plan: PlanRect,
	) -> Option<RectFloorSlabGeom> {
		resolve_horizontal_slab(self.style, plan, slab, &self.openings, self.joint_thickness)
	}
}

pub(crate) fn resolve_horizontal_slab(
	style: PanelStyle,
	plan: PlanRect,
	slab: RectFloorSlab,
	openings: &Openings,
	thickness: f32,
) -> Option<RectFloorSlabGeom> {
	match slab {
		RectFloorSlab::None => None,
		RectFloorSlab::Solid => {
			let cutting =
				openings.iter().filter_map(
					|(_id, o)| {
						if o.label.cuts_slab() {
							Some(o.bounds)
						} else {
							None
						}
					},
				);
			match merge_slab_insets(plan, cutting) {
				None => Some(RectFloorSlabGeom::Solid(solid_slab(style, plan, thickness))),
				Some(None) => None,
				Some(Some(inset)) => {
					Some(RectFloorSlabGeom::Clipped(clipped_slab(style, plan, thickness, inset)))
				}
			}
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

fn solid_slab(style: PanelStyle, plan: PlanRect, thickness: f32) -> FittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	FittedRectangle::new(style, sw, se, nw, ne)
}

fn clipped_slab(
	style: PanelStyle,
	plan: PlanRect,
	thickness: f32,
	inset: RectInset,
) -> ClippedFittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	ClippedFittedRectangle::new(style, sw, se, nw, ne, inset)
}
