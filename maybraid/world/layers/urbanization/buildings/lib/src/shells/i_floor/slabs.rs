//! Multi-rect floor / ceiling with positioned cuts_slab holes.

use building_components::panels::PanelStyle;

use crate::openings::Openings;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::PanelPoint;
use crate::paneling::RectInset;
use crate::shells::ortho::{merge_slab_insets, PlanRect};

use super::geometry::PlanAabb;
use super::{IFloorParams, IFloorSlab, ISlabPiece};

impl IFloorParams {
	pub(super) fn resolve_slab_pieces(
		&self,
		slab: IFloorSlab,
		rects: &[PlanAabb],
		y: f32,
	) -> Vec<ISlabPiece> {
		match slab {
			IFloorSlab::None => Vec::new(),
			IFloorSlab::Solid => {
				let mut out = Vec::new();
				for r in rects {
					let plan = r.to_plan_rect(y);
					if let Some(piece) =
						resolve_piece(self.style, plan, &self.openings, self.joint_thickness)
					{
						out.push(piece);
					}
				}
				out
			}
		}
	}
}

fn resolve_piece(
	style: PanelStyle,
	plan: PlanRect,
	openings: &Openings,
	thickness: f32,
) -> Option<ISlabPiece> {
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
		None => Some(ISlabPiece::Solid(solid(style, plan, thickness))),
		Some(None) => None,
		Some(Some(inset)) => Some(ISlabPiece::Clipped(clipped(style, plan, thickness, inset))),
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

fn solid(style: PanelStyle, plan: PlanRect, thickness: f32) -> FittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	FittedRectangle::new(style, sw, se, nw, ne)
}

fn clipped(
	style: PanelStyle,
	plan: PlanRect,
	thickness: f32,
	inset: RectInset,
) -> ClippedFittedRectangle {
	let [sw, se, nw, ne] = corners(plan, thickness);
	ClippedFittedRectangle::new(style, sw, se, nw, ne, inset)
}
