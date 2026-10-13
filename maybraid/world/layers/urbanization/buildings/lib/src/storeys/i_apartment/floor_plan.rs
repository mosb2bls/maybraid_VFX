//! I-Apartment floor plan: fit an [`IFloor`], cut façade windows, map shafts.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::labels::{LabelNode, LabelStyle};
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{plan_to_aabb3, PlanAxes};

use crate::fit::{
	aabb_xz_center, aabb_xz_extent, aabb_xz_overlap_area, Confines, FillRegion, FillableRegions,
	Fit, FitError, SpaceKind, StackRegion,
};
use crate::openings::{
	sync_connectable_openings_from_mapped, Opening, OpeningId, OpeningLabel, Openings,
};
use crate::shells::ortho::EPS;
use crate::shells::{IFloor, IFloorParams, IFloorPlanRect, IFloorSlab};

use super::parameterized::{
	IApartmentParameterized, MIN_CENTRAL_DEPTH, MIN_POCKET_SHAFT_SIDE, MIN_STOREY_HEIGHT,
};
use super::SCOPE;

/// Keep shaft volumes clear of IFloor wall strips (panel thickness + jamb margin).
const SHAFT_WALL_CLEARANCE: f32 = 0.75;

/// I-Apartment floor plan: I-frame shell + primary rectangular regions + openings.
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentFloorPlan {
	pub parameterized: IApartmentParameterized,
	pub center_xz: Vec3,
	pub storey_height: f32,
	pub roll: f32,
	/// Corridor clear width used for inter-rect passages and HallsToShafts.
	pub hall_width: f32,
	pub shell: IFloor,
	/// Natural I-frame rectangles (stem + optional flange bars), 1–3.
	pub primary_rects: Vec<IFloorPlanRect>,
	/// Merged inbound + generated openings (post shell sync for connectables).
	pub openings: Openings,
	/// Active shaft volumes (only 9-pocket slots that received inbound shafts).
	pub shaft_bounds: Vec<Aabb3d>,
	/// Stable slot key `rect_index * 9 + pocket_index` for each [`Self::shaft_bounds`].
	pub shaft_slots: Vec<usize>,
	/// Inbound shaft [`OpeningId`]s remapped onto each active shaft (parallel).
	pub shaft_inbound: Vec<Vec<OpeningId>>,
}

impl IApartmentFloorPlan {
	pub fn from_parameterized(
		params: IApartmentParameterized,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_parameterized_with_ceiling(params, confines, IFloorSlab::None)
	}

	pub fn from_parameterized_with_ceiling(
		params: IApartmentParameterized,
		confines: &Confines,
		ceiling: IFloorSlab,
	) -> Result<(Self, FillableRegions), FitError> {
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let y0 = confines.bounds.min.y;
		let center = confines.center();
		let center_xz = Vec3::new(center.x, y0, center.z);

		let mut ifloor_params =
			derive_ifloor_params(&params, confines, center_xz, height, ceiling)?;
		let primary_rects = ifloor_params.plan_rects();
		if primary_rects.is_empty() {
			return Err(FitError::TooSmall { reason: "i_rects" });
		}

		let mut openings = confines.openings.clone();
		let (shaft_bounds, shaft_slots, shaft_inbound) =
			map_inbound_shafts(&mut openings, &primary_rects, height, y0, &params);

		// Authored shaft volumes (slab cuts) for each active pocket.
		for (i, shaft) in shaft_bounds.iter().enumerate() {
			let slot = shaft_slots.get(i).copied().unwrap_or(i);
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft", slot.to_string()),
				Opening::new(*shaft, OpeningLabel::Shaft),
			);
		}

		openings.extend(&generated_facade_apertures(&params, &ifloor_params, height));

		ifloor_params.openings = openings.clone();
		let shell = IFloor::new(ifloor_params);
		sync_connectable_openings_from_mapped(&mut openings, &shell);
		// Interior connectivity: after sync so Passage sync does not drop them.
		openings.extend(&inter_rect_passages(&primary_rects, params.hall_width, height, y0));

		let plan = Self {
			parameterized: params.clone(),
			center_xz,
			storey_height: height,
			roll: confines.roll,
			hall_width: params.hall_width,
			shell,
			primary_rects,
			openings,
			shaft_bounds,
			shaft_slots,
			shaft_inbound,
		};
		let regions = plan.fillable_regions();
		Ok((plan, regions))
	}

	/// Small inbound shaft requests inside each primary rect (playground / tests).
	pub fn shaft_requests_for_primary_rects(
		params: &IApartmentParameterized,
		confines: &Confines,
	) -> Openings {
		let Ok(ifloor) = derive_ifloor_params(
			params,
			confines,
			Vec3::new(confines.center().x, confines.bounds.min.y, confines.center().z),
			(confines.bounds.max.y - confines.bounds.min.y).max(0.0),
			IFloorSlab::None,
		) else {
			return Openings::new();
		};
		let y0 = confines.bounds.min.y;
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0).min(3.0);
		let mut openings = Openings::new();
		for (ri, rect) in ifloor.plan_rects().iter().enumerate() {
			// Prefer a boundary pocket (SW) so demos exercise non-center slots.
			let pockets = nine_pockets(rect);
			let pocket = &pockets[0];
			let cx = (pocket.min.x + pocket.max.x) * 0.5;
			let cz = (pocket.min.y + pocket.max.y) * 0.5;
			let half = 0.4_f32;
			openings.insert(
				OpeningId::scoped(SCOPE, "shaft_req", ri.to_string()),
				Opening::new(
					Aabb3d::from_min_max(
						Vec3::new(cx - half, y0, cz - half),
						Vec3::new(cx + half, y0 + height, cz + half),
					),
					OpeningLabel::Shaft,
				),
			);
		}
		openings
	}

	/// Primary I-frame rectangles as typed residuals (one confine per rect).
	pub fn fillable_regions(&self) -> FillableRegions {
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		let host = Aabb3d::from_min_max(
			Vec3::new(
				self.primary_rects.iter().map(|r| r.min_x).fold(f32::INFINITY, f32::min),
				y0,
				self.primary_rects.iter().map(|r| r.min_z).fold(f32::INFINITY, f32::min),
			),
			Vec3::new(
				self.primary_rects.iter().map(|r| r.max_x).fold(f32::NEG_INFINITY, f32::max),
				y1,
				self.primary_rects.iter().map(|r| r.max_z).fold(f32::NEG_INFINITY, f32::max),
			),
		);

		let mut within = Vec::new();
		let rects2: Vec<Aabb2d> = self.primary_rects.iter().map(|r| r.to_aabb2()).collect();
		let y0 = Vec3::from(host.min).y;
		let y1 = Vec3::from(host.max).y;
		for (i, _rect) in self.primary_rects.iter().enumerate() {
			let rect2 = rects2[i];
			let bounds = plan_to_aabb3(&host, rect2, PlanAxes::XZ);
			let mut openings = openings_intersecting_xz(&self.openings, rect2);
			// Shell owns true exterior; leave shared primary-rect edges free so the
			// first progressive fill can box them, then hand Boundary to siblings.
			let siblings: Vec<Aabb2d> =
				rects2.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, r)| *r).collect();
			crate::usage_areas::boundary_openings::inject_exterior_boundaries(
				rect2,
				&siblings,
				y0,
				y1,
				SCOPE,
				&format!("rect_{i}"),
				&mut openings,
			);
			within.push(FillRegion::new(
				SpaceKind::Custom(format!("{SCOPE}_rect_{i}")),
				Confines::new(bounds, self.roll, openings),
			));
		}

		let atop_bounds = primary_union_aabb2(&self.primary_rects).unwrap_or(Aabb2d {
			min: Vec2::new(self.center_xz.x - 1.0, self.center_xz.z - 1.0),
			max: Vec2::new(self.center_xz.x + 1.0, self.center_xz.z + 1.0),
		});
		let atop = vec![StackRegion {
			bounds: atop_bounds,
			height: self.storey_height,
			roll: self.roll,
			openings: self.openings.clone(),
		}];

		FillableRegions { within, atop }
	}
}

impl Fit for IApartmentFloorPlan {
	fn fit_to_confines(
		confines: &Confines,
		noise: procedural_common::NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = IApartmentParameterized::sample(confines, noise)?;
		Self::from_parameterized(params, confines)
	}
}

impl BuildingComponents for IApartmentFloorPlan {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		let y0 = self.center_xz.y;
		let y1 = y0 + self.storey_height;
		for (i, rect) in self.primary_rects.iter().enumerate() {
			let bounds = Aabb3d::from_min_max(
				Vec3::new(rect.min_x, y0, rect.min_z),
				Vec3::new(rect.max_x, y1, rect.max_z),
			);
			let label = match (i, self.primary_rects.len()) {
				(0, _) => "Stem",
				(1, 2) => "Flange",
				(1, _) => "Top flange",
				(2, _) => "Bottom flange",
				_ => "Rect",
			};
			out.push_free(label_filling_aabb(
				LabelStyle::Blue,
				&format!("{label} {i}"),
				&bounds,
				self.roll,
			));
		}
		out
	}
}

/// Remap inbound shafts onto 9-pocket centroids; drop requests outside all primary rects.
fn map_inbound_shafts(
	openings: &mut Openings,
	primary_rects: &[IFloorPlanRect],
	height: f32,
	y0: f32,
	params: &IApartmentParameterized,
) -> (Vec<Aabb3d>, Vec<usize>, Vec<Vec<OpeningId>>) {
	use std::collections::BTreeMap;

	let shaft_ids: Vec<OpeningId> = openings
		.iter()
		.filter(|(_, o)| matches!(o.label, OpeningLabel::Shaft))
		.map(|(id, _)| id.clone())
		.collect();

	let mut by_slot: BTreeMap<usize, Vec<OpeningId>> = BTreeMap::new();
	let mut slot_bounds: BTreeMap<usize, Aabb3d> = BTreeMap::new();

	for id in shaft_ids {
		let Some(opening) = openings.get(&id).cloned() else {
			continue;
		};
		let rc = aabb_xz_center(&opening.bounds);
		let Some(rect_i) = primary_rects.iter().position(|r| rect_contains_xz(r, rc)) else {
			openings.openings.remove(&id);
			continue;
		};
		let pockets = nine_pockets(&primary_rects[rect_i]);
		let Some(pocket_i) = best_pocket(&opening.bounds, &pockets) else {
			openings.openings.remove(&id);
			continue;
		};
		let slot = rect_i * 9 + pocket_i;
		let shaft =
			shaft_aabb_at_pocket(&primary_rects[rect_i], &pockets[pocket_i], y0, height, params);
		openings.insert(id.clone(), Opening::new(shaft, OpeningLabel::Shaft));
		by_slot.entry(slot).or_default().push(id);
		slot_bounds.entry(slot).or_insert(shaft);
	}

	let mut shaft_bounds = Vec::new();
	let mut shaft_slots = Vec::new();
	let mut shaft_inbound = Vec::new();
	for (slot, ids) in by_slot {
		if ids.is_empty() {
			continue;
		}
		shaft_slots.push(slot);
		shaft_bounds.push(slot_bounds[&slot]);
		shaft_inbound.push(ids);
	}
	(shaft_bounds, shaft_slots, shaft_inbound)
}

fn generated_facade_apertures(
	params: &IApartmentParameterized,
	ifloor_params: &IFloorParams,
	height: f32,
) -> Openings {
	let mut openings = Openings::new();
	let win_h = (height * 0.42).clamp(1.0, height.max(1.0));
	let sill = (height * 0.28).clamp(0.7, height * 0.45);
	let edges = ifloor_params.wall_edges();
	for (ei, edge) in edges.iter().enumerate() {
		let run = edge.length();
		if run < 1.5 {
			continue;
		}
		let placed = params.fit_windows_on_run(run);
		for (wi, win) in placed.iter().enumerate() {
			let along_mid = win.along + win.width * 0.5;
			openings.insert(
				OpeningId::scoped(SCOPE, "outer_aperture", format!("{ei}_{wi}")),
				IFloor::edge_aperture_opening_at(*edge, along_mid, win.width, win_h, sill),
			);
		}
	}
	openings
}

/// 3×3 thirds partition of a primary rect (row-major, −Z → +Z, −X → +X).
fn nine_pockets(rect: &IFloorPlanRect) -> Vec<Aabb2d> {
	let dx = ((rect.max_x - rect.min_x) / 3.0).max(EPS);
	let dz = ((rect.max_z - rect.min_z) / 3.0).max(EPS);
	let mut out = Vec::with_capacity(9);
	for jz in 0..3 {
		for ix in 0..3 {
			out.push(Aabb2d {
				min: Vec2::new(rect.min_x + ix as f32 * dx, rect.min_z + jz as f32 * dz),
				max: Vec2::new(
					rect.min_x + (ix + 1) as f32 * dx,
					rect.min_z + (jz + 1) as f32 * dz,
				),
			});
		}
	}
	out
}

fn rect_contains_xz(rect: &IFloorPlanRect, p: Vec2) -> bool {
	p.x >= rect.min_x - EPS
		&& p.x <= rect.max_x + EPS
		&& p.y >= rect.min_z - EPS
		&& p.y <= rect.max_z + EPS
}

fn best_pocket(request: &Aabb3d, pockets: &[Aabb2d]) -> Option<usize> {
	if pockets.is_empty() {
		return None;
	}
	let rc = aabb_xz_center(request);
	let mut best_i = 0usize;
	let mut best_area = -1.0_f32;
	let mut best_dist = f32::INFINITY;
	for (i, pocket) in pockets.iter().enumerate() {
		let area = aabb_xz_overlap_area(request, pocket);
		let cx = (pocket.min.x + pocket.max.x) * 0.5;
		let cz = (pocket.min.y + pocket.max.y) * 0.5;
		let dist = (rc.x - cx).hypot(rc.y - cz);
		let better = area > best_area + 1e-6
			|| ((area - best_area).abs() <= 1e-6 && dist < best_dist - 1e-6);
		if better {
			best_i = i;
			best_area = area;
			best_dist = dist;
		}
	}
	Some(best_i)
}

fn shaft_aabb_at_pocket(
	rect: &IFloorPlanRect,
	pocket: &Aabb2d,
	y0: f32,
	height: f32,
	params: &IApartmentParameterized,
) -> Aabb3d {
	let mut cx = (pocket.min.x + pocket.max.x) * 0.5;
	let mut cz = (pocket.min.y + pocket.max.y) * 0.5;
	let pw = (pocket.max.x - pocket.min.x).max(EPS);
	let pd = (pocket.max.y - pocket.min.y).max(EPS);
	// Inset from pocket edges so boundary pockets never sit on / punch outer walls.
	let clear = SHAFT_WALL_CLEARANCE.min(pw * 0.35).min(pd * 0.35);
	let floor_half = MIN_POCKET_SHAFT_SIDE * 0.5;
	let max_half_x = ((pw * 0.5) - clear).max(floor_half);
	let max_half_z = ((pd * 0.5) - clear).max(floor_half);
	let pocket_half = (params.shaft_side * 0.5).max(0.6).min(max_half_x).min(max_half_z);
	// A kept floor may outgrow the pocket, but never the rect's wall clearance.
	let room_x = ((rect.max_x - rect.min_x) * 0.5 - SHAFT_WALL_CLEARANCE).max(pocket_half);
	let room_z = ((rect.max_z - rect.min_z) * 0.5 - SHAFT_WALL_CLEARANCE).max(pocket_half);
	let half = pocket_half.max(params.min_shaft_side * 0.5).min(room_x).min(room_z);
	if half > pocket_half + EPS {
		let slide = |c: f32, lo: f32, hi: f32| {
			let (lo, hi) = (lo + SHAFT_WALL_CLEARANCE + half, hi - SHAFT_WALL_CLEARANCE - half);
			if lo <= hi {
				c.clamp(lo, hi)
			} else {
				(lo + hi) * 0.5
			}
		};
		cx = slide(cx, rect.min_x, rect.max_x);
		cz = slide(cz, rect.min_z, rect.max_z);
	}
	Aabb3d::from_min_max(
		Vec3::new(cx - half, y0, cz - half),
		Vec3::new(cx + half, y0 + height.max(EPS), cz + half),
	)
}

fn derive_ifloor_params(
	params: &IApartmentParameterized,
	confines: &Confines,
	center_xz: Vec3,
	height: f32,
	ceiling: IFloorSlab,
) -> Result<IFloorParams, FitError> {
	let footprint = aabb_xz_extent(&confines.bounds);
	let stem_w = params.stem_width.clamp(1.0, footprint.x * 0.9);
	let flange_bars = (params.has_top_flange() as u8) + (params.has_bottom_flange() as u8);
	let flange_t = if flange_bars == 0 {
		params.flange_thickness
	} else {
		let max_t = ((footprint.y - MIN_CENTRAL_DEPTH) / flange_bars as f32).max(0.5);
		params.flange_thickness.min(max_t)
	};
	let min_depth = flange_t * flange_bars as f32 + MIN_CENTRAL_DEPTH;
	if footprint.y < min_depth {
		return Err(FitError::TooSmall { reason: "stem_depth" });
	}
	let central_depth = footprint.y - flange_t * flange_bars as f32;
	if central_depth < MIN_CENTRAL_DEPTH {
		return Err(FitError::TooSmall { reason: "stem_depth" });
	}
	let (tl, tr, bl, br) = params.flange_lengths(footprint, stem_w);
	Ok(IFloorParams {
		center_xz,
		top_left_length: tl,
		top_right_length: tr,
		central_rectangle: Vec2::new(stem_w, central_depth),
		bottom_left_length: bl,
		bottom_right_length: br,
		flange_thickness: Some(flange_t),
		storey_height: height,
		openings: Openings::new(),
		floor: IFloorSlab::Solid,
		ceiling,
		style: PanelStyle::RoughStonework,
		joint_thickness: crate::paneling::DEFAULT_PANEL_THICKNESS,
	})
}

fn primary_union_aabb2(rects: &[IFloorPlanRect]) -> Option<Aabb2d> {
	let mut iter = rects.iter();
	let first = iter.next()?;
	let mut min = Vec2::new(first.min_x, first.min_z);
	let mut max = Vec2::new(first.max_x, first.max_z);
	for r in iter {
		min.x = min.x.min(r.min_x);
		min.y = min.y.min(r.min_z);
		max.x = max.x.max(r.max_x);
		max.y = max.y.max(r.max_z);
	}
	Some(Aabb2d { min, max })
}

fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, _yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, 0.0)
}

fn openings_intersecting_xz(openings: &Openings, region: Aabb2d) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if aabb_xz_overlap_area(&opening.bounds, &region) > EPS {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

/// Hall-width passages on shared edges between primary rects (connectivity terminals).
fn inter_rect_passages(
	rects: &[IFloorPlanRect],
	hall_width: f32,
	height: f32,
	y0: f32,
) -> Openings {
	use crate::paneling::DEFAULT_PANEL_THICKNESS;
	const JAMB: f32 = 0.35;
	let depth = (DEFAULT_PANEL_THICKNESS + 0.08).max(0.2);
	let half_d = depth * 0.5;
	let mut out = Openings::new();
	for i in 0..rects.len() {
		for j in (i + 1)..rects.len() {
			let a = &rects[i];
			let b = &rects[j];
			let Some((along_x, mid, lo, hi)) = shared_edge_span(a, b) else {
				continue;
			};
			let shared_len = (hi - lo).max(0.0);
			let clear = hall_width.min((shared_len - 2.0 * JAMB).max(0.0));
			if clear < 0.8 {
				continue;
			}
			let center = 0.5 * (lo + hi);
			let half_c = clear * 0.5;
			let door_lo = (center - half_c).max(lo);
			let door_hi = (center + half_c).min(hi);
			let bounds = if along_x {
				// Shared edge runs along X; straddle in Z.
				Aabb3d::from_min_max(
					Vec3::new(door_lo, y0, mid - half_d),
					Vec3::new(door_hi, y0 + height.min(2.4), mid + half_d),
				)
			} else {
				Aabb3d::from_min_max(
					Vec3::new(mid - half_d, y0, door_lo),
					Vec3::new(mid + half_d, y0 + height.min(2.4), door_hi),
				)
			};
			out.insert(
				OpeningId::scoped(SCOPE, "inter_rect", format!("{i}_{j}")),
				Opening::new(bounds, OpeningLabel::Passage),
			);
		}
	}
	out
}

/// Shared edge between two plan rects: `(along_x, mid_perp, lo_along, hi_along)`.
fn shared_edge_span(a: &IFloorPlanRect, b: &IFloorPlanRect) -> Option<(bool, f32, f32, f32)> {
	// Vertical joint (shared X plane): overlap in Z.
	if (a.max_x - b.min_x).abs() <= EPS || (b.max_x - a.min_x).abs() <= EPS {
		let mid = if (a.max_x - b.min_x).abs() <= EPS { a.max_x } else { b.max_x };
		let lo = a.min_z.max(b.min_z);
		let hi = a.max_z.min(b.max_z);
		if hi - lo > EPS {
			return Some((false, mid, lo, hi));
		}
	}
	// Horizontal joint (shared Z plane): overlap in X.
	if (a.max_z - b.min_z).abs() <= EPS || (b.max_z - a.min_z).abs() <= EPS {
		let mid = if (a.max_z - b.min_z).abs() <= EPS { a.max_z } else { b.max_z };
		let lo = a.min_x.max(b.min_x);
		let hi = a.max_x.min(b.max_x);
		if hi - lo > EPS {
			return Some((true, mid, lo, hi));
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::MapsOpenings;
	use procedural_common::NoiseParams;

	fn large_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		))
	}

	#[test]
	fn emits_one_to_three_primary_rects() {
		let confines = large_confines();
		let params = IApartmentParameterized::sample(&confines, NoiseParams::default()).unwrap();
		let (plan, regions) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!((1..=3).contains(&plan.primary_rects.len()));
		assert_eq!(regions.within.len(), plan.primary_rects.len());
		assert!(plan.hall_width >= crate::usage_areas::MIN_HALL_WIDTH - 1e-3);
		assert!(!plan.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn multi_rect_authors_inter_rect_passages() {
		let confines = large_confines();
		let mut saw = false;
		for seed in 0..80 {
			let params = IApartmentParameterized::sample(
				&confines,
				NoiseParams { seed, ..NoiseParams::default() },
			)
			.unwrap();
			let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
			if plan.primary_rects.len() < 2 {
				continue;
			}
			let n = plan
				.openings
				.iter()
				.filter(|(id, o)| {
					id.as_str().contains("inter_rect") && matches!(o.label, OpeningLabel::Passage)
				})
				.count();
			if n > 0 {
				saw = true;
				break;
			}
		}
		assert!(saw, "expected inter_rect passages on a multi-rect plan");
	}

	#[test]
	fn facade_apertures_are_authored() {
		let confines = large_confines();
		let params = IApartmentParameterized::sample(&confines, NoiseParams::default()).unwrap();
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		let windows = plan
			.openings
			.iter()
			.filter(|(_, o)| matches!(o.label, OpeningLabel::Aperture))
			.count();
		assert!(windows > 0, "expected exterior apertures");
		assert!(
			plan.shell
				.openings()
				.iter()
				.any(|(_, o)| matches!(o.label, OpeningLabel::Aperture)),
			"shell should map some apertures"
		);
	}

	#[test]
	fn shafts_map_to_nine_pocket_centroids() {
		let empty = large_confines();
		let params = IApartmentParameterized::sample(&empty, NoiseParams::default()).unwrap();
		let inbound = IApartmentFloorPlan::shaft_requests_for_primary_rects(&params, &empty);
		assert!(!inbound.openings.is_empty());
		let confines = Confines::new(empty.bounds, 0.0, inbound);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert_eq!(plan.shaft_bounds.len(), plan.primary_rects.len());
		assert_eq!(plan.shaft_bounds.len(), plan.shaft_slots.len());
		for (shaft, slot) in plan.shaft_bounds.iter().zip(plan.shaft_slots.iter()) {
			let rect_i = slot / 9;
			let pocket_i = slot % 9;
			let pockets = nine_pockets(&plan.primary_rects[rect_i]);
			let pocket = &pockets[pocket_i];
			let c = aabb_xz_center(shaft);
			let pc =
				Vec2::new((pocket.min.x + pocket.max.x) * 0.5, (pocket.min.y + pocket.max.y) * 0.5);
			assert!(
				(c.x - pc.x).abs() < 1e-2 && (c.y - pc.y).abs() < 1e-2,
				"shaft center should sit on pocket centroid"
			);
			let smin = Vec3::from(shaft.min);
			let smax = Vec3::from(shaft.max);
			let gap_x = (smin.x - pocket.min.x).min(pocket.max.x - smax.x);
			let gap_z = (smin.z - pocket.min.y).min(pocket.max.y - smax.z);
			assert!(
				gap_x > 0.25 && gap_z > 0.25,
				"shaft should stay inset from pocket / wall edges (gaps {gap_x}, {gap_z})"
			);
		}
		// Boundary shafts must not map onto exterior wall strips.
		assert!(
			!plan
				.shell
				.openings()
				.iter()
				.any(|(_, o)| matches!(o.label, OpeningLabel::Shaft)),
			"shaft should not punch wall openings"
		);
	}

	#[test]
	fn shafts_outside_primary_rects_are_dropped() {
		let empty = large_confines();
		let params = IApartmentParameterized::sample(&empty, NoiseParams::default()).unwrap();
		let mut openings = Openings::new();
		openings.insert(
			"far_shaft",
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(100.0, 0.0, 100.0), Vec3::new(101.0, 3.0, 101.0)),
				OpeningLabel::Shaft,
			),
		);
		let confines = Confines::new(empty.bounds, 0.0, openings);
		let (plan, _) = IApartmentFloorPlan::from_parameterized(params, &confines).unwrap();
		assert!(plan.shaft_bounds.is_empty());
		assert!(!plan.openings.openings.contains_key(&OpeningId::new("far_shaft")));
	}
}
