//! One commercial stall bay: shell walls + catalog interior.
//!
//! **Semantically:** a gallery shop cell with punched passages / apertures on the
//! envelope and a typed interior (food, retail, restroom, …).
//!
//! **Programmatically:**
//! 1. [`CommercialStallParameterized::sample`] → style knobs from noise.
//! 2. [`CommercialStallPlan::from_parameterized`] builds clipped wall strips from
//!    confines openings, then fits [`CommercialStallInterior`] on a confines copy
//!    that forwards only Passage / Aperture openings.
//! 3. [`CommercialStall`] is the presentable shell + interior; residual
//!    [`FillableRegions`] come from the interior (e.g. office / stalls rooms).

mod bites_sitdown_stall;
mod bites_stall;
mod interior;
mod knick_knack_stall;
mod label_util;
mod lounge;
mod mini_mart;
mod parts_stall;
mod public_restroom;
mod stall_layout;

pub use bites_sitdown_stall::{BitesSitdownParameterized, BitesSitdownPlan, BitesSitdownStall};
pub use bites_stall::{BitesStall, BitesStallParameterized, BitesStallPlan};
pub use interior::CommercialStallInterior;
pub use knick_knack_stall::{KnickKnackStall, KnickKnackStallParameterized, KnickKnackStallPlan};
pub use lounge::Lounge;
pub use mini_mart::{MiniMart, MiniMartParameterized, MiniMartPlan};
pub use parts_stall::{PartsStall, PartsStallParameterized, PartsStallPlan};
pub use public_restroom::{PublicRestroom, PublicRestroomParameterized, PublicRestroomPlan};

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::furniture::FurnitureNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::constraints::FaceKind;
use crate::fit::{aabb_near_plane, aabb_xz_overlap_area, Confines, FillableRegions, Fit, FitError};
use crate::openings::{OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::ortho::{standing_face_opening, WallEdge, EPS};

const MIN_STALL: f32 = 1.2;

/// Noise knobs for a commercial stall shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallParameterized {
	pub style: LabelStyle,
}

impl CommercialStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 11.0);
		Self { style: LabelStyle::from_unit(t) }
	}
}

/// Stall plan: walls + interior.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallPlan {
	pub parameterized: CommercialStallParameterized,
	pub walls: Vec<ClippedRectangularStrip>,
	pub interior: CommercialStallInterior,
}

impl CommercialStallPlan {
	pub fn from_parameterized(
		params: CommercialStallParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = (max - min).max(Vec3::splat(1e-4));
		if extent.x < MIN_STALL || extent.z < MIN_STALL || extent.y < 0.8 {
			return Err(FitError::TooSmall { reason: "stall" });
		}
		let walls = shell_walls(confines);
		let interior_confines =
			Confines::new(confines.bounds, confines.roll, forward_openings(&confines.openings));
		let (interior, regions) =
			CommercialStallInterior::fit_to_confines(&interior_confines, noise)?;
		Ok((Self { parameterized: params, walls, interior }, regions))
	}
}

/// Full commercial stall.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStall {
	pub plan: CommercialStallPlan,
	/// Bay the stall claimed — used to drop a residual chest in empty lounges.
	pub confines: Confines,
}

impl CommercialStall {
	pub fn from_plan(plan: CommercialStallPlan, confines: Confines) -> Self {
		Self { plan, confines }
	}

	pub fn interior(&self) -> &CommercialStallInterior {
		&self.plan.interior
	}
}

impl Fit for CommercialStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallParameterized::sample(confines, noise);
		let (plan, regions) = CommercialStallPlan::from_parameterized(params, confines, noise)?;
		Ok((Self::from_plan(plan, confines.clone()), regions))
	}
}

impl BuildingComponents for CommercialStall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.plan.walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out.extend(self.plan.interior.panel_nodes_for_level(level));
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.plan.interior.label_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.plan.interior.furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		self.plan.interior.furniture_usage_nodes_for_level(level)
	}
}

fn shell_walls(confines: &Confines) -> Vec<ClippedRectangularStrip> {
	let mut walls = Vec::new();
	for face in [FaceKind::Front, FaceKind::Back, FaceKind::Left, FaceKind::Right] {
		if face_excluded(&confines.bounds, face, &confines.openings) {
			continue;
		}
		let Some(edge) = face_edge(&confines.bounds, face) else {
			continue;
		};
		walls.push(wall_strip_for_face(edge, &confines.openings));
	}
	walls
}

fn face_edge(bounds: &Aabb3d, face: FaceKind) -> Option<WallEdge> {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let h = (max.y - min.y).max(EPS);
	let (start, end, outward) = match face {
		FaceKind::Front => {
			(Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z), Vec2::new(0.0, -1.0))
		}
		FaceKind::Back => {
			(Vec3::new(min.x, min.y, max.z), Vec3::new(max.x, min.y, max.z), Vec2::new(0.0, 1.0))
		}
		FaceKind::Left => {
			(Vec3::new(min.x, min.y, min.z), Vec3::new(min.x, min.y, max.z), Vec2::new(-1.0, 0.0))
		}
		FaceKind::Right => {
			(Vec3::new(max.x, min.y, min.z), Vec3::new(max.x, min.y, max.z), Vec2::new(1.0, 0.0))
		}
		FaceKind::Top | FaceKind::Bottom => return None,
	};
	if start.distance(end) < EPS {
		return None;
	}
	Some(WallEdge::new(start, end, h, outward))
}

fn wall_strip_for_face(edge: WallEdge, openings: &Openings) -> ClippedRectangularStrip {
	let thickness = DEFAULT_PANEL_THICKNESS;
	let mut assigned = Vec::new();
	for (id, opening) in openings.iter() {
		if !matches!(
			opening.label,
			OpeningLabel::Passage | OpeningLabel::Aperture | OpeningLabel::Shaft
		) {
			continue;
		}
		let Some(face) = standing_face_opening(edge, &opening.bounds, thickness) else {
			continue;
		};
		let len = edge.length();
		let s_lo = face.inset.bottom.clamp(0.0, len);
		let s_hi = (len - face.inset.top).clamp(0.0, len);
		if s_hi - s_lo < EPS {
			continue;
		}
		assigned.push(FaceCut {
			_id: id.clone(),
			s_lo,
			s_hi,
			sill: face.inset.left,
			header: face.inset.right,
			priority: connectable_priority(&opening.label),
		});
	}
	assigned.sort_by(|a, b| {
		b.priority
			.cmp(&a.priority)
			.then_with(|| a.s_lo.partial_cmp(&b.s_lo).unwrap_or(std::cmp::Ordering::Equal))
	});
	let mut kept = Vec::new();
	for op in assigned {
		if kept.iter().any(|k: &FaceCut| spans_overlap(k.s_lo, k.s_hi, op.s_lo, op.s_hi)) {
			continue;
		}
		kept.push(op);
	}
	kept.sort_by(|a, b| a.s_lo.partial_cmp(&b.s_lo).unwrap_or(std::cmp::Ordering::Equal));
	build_wall_strip(edge, &kept, thickness)
}

struct FaceCut {
	_id: crate::openings::OpeningId,
	s_lo: f32,
	s_hi: f32,
	sill: f32,
	header: f32,
	priority: u8,
}

fn connectable_priority(label: &OpeningLabel) -> u8 {
	match label {
		OpeningLabel::Passage | OpeningLabel::Shaft => 2,
		OpeningLabel::Aperture => 1,
		_ => 0,
	}
}

fn spans_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> bool {
	a0 < b1 - EPS && b0 < a1 - EPS
}

fn build_wall_strip(
	edge: WallEdge,
	openings: &[FaceCut],
	thickness: f32,
) -> ClippedRectangularStrip {
	let len = edge.length();
	let tang = edge.tangent();
	let h = edge.height;
	let t = thickness.max(1e-4);
	let style = PanelStyle::RoughStonework;

	if openings.is_empty() {
		return ClippedRectangularStrip::from_nodes(
			style,
			[
				RectangularStripNode::new(edge.start, h, t, 0.0),
				RectangularStripNode::new(edge.end, h, t, 0.0),
			],
			[None],
		);
	}

	let mut nodes = Vec::new();
	let mut insets: Vec<Option<RectInset>> = Vec::new();
	nodes.push(RectangularStripNode::new(edge.start, h, t, 0.0));
	let mut cursor = 0.0_f32;

	for op in openings {
		if op.s_lo > cursor + EPS {
			nodes.push(RectangularStripNode::new(edge.start + tang * op.s_lo, h, t, 0.0));
			insets.push(None);
			cursor = op.s_lo;
		}
		let s_hi = op.s_hi.max(cursor + EPS);
		nodes.push(RectangularStripNode::new(edge.start + tang * s_hi, h, t, 0.0));
		// Standing strip: left/right = vertical, bottom/top = along-wall jambs.
		let jamb = 0.02_f32.min((s_hi - cursor) * 0.1);
		insets.push(Some(RectInset::new(op.sill, op.header, jamb, jamb)));
		cursor = s_hi;
	}

	if cursor < len - EPS {
		nodes.push(RectangularStripNode::new(edge.end, h, t, 0.0));
		insets.push(None);
	} else if let Some(last) = nodes.last_mut() {
		last.position = edge.end;
	}

	ClippedRectangularStrip::from_nodes(style, nodes, insets)
}

fn face_excluded(bounds: &Aabb3d, face: FaceKind, openings: &Openings) -> bool {
	for (_id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Exclusion | OpeningLabel::Boundary) {
			continue;
		}
		if opening_covers_face(bounds, face, &opening.bounds) {
			return true;
		}
	}
	false
}

fn opening_covers_face(bounds: &Aabb3d, face: FaceKind, opening: &Aabb3d) -> bool {
	let bmin = Vec3::from(bounds.min);
	let bmax = Vec3::from(bounds.max);
	let omin = Vec3::from(opening.min);
	let omax = Vec3::from(opening.max);
	let tol = 0.4_f32;
	let on_plane = match face {
		FaceKind::Front => aabb_near_plane(omin.z, omax.z, bmin.z, tol),
		FaceKind::Back => aabb_near_plane(omin.z, omax.z, bmax.z, tol),
		FaceKind::Right => aabb_near_plane(omin.x, omax.x, bmax.x, tol),
		FaceKind::Left => aabb_near_plane(omin.x, omax.x, bmin.x, tol),
		FaceKind::Top | FaceKind::Bottom => return false,
	};
	if !on_plane {
		return false;
	}
	let face_region = match face {
		FaceKind::Front | FaceKind::Back => bevy_math::bounding::Aabb2d {
			min: bevy_math::Vec2::new(bmin.x, bmin.z - 0.5),
			max: bevy_math::Vec2::new(bmax.x, bmax.z + 0.5),
		},
		FaceKind::Left | FaceKind::Right => bevy_math::bounding::Aabb2d {
			min: bevy_math::Vec2::new(bmin.x - 0.5, bmin.z),
			max: bevy_math::Vec2::new(bmax.x + 0.5, bmax.z),
		},
		_ => return false,
	};
	let face_len = match face {
		FaceKind::Front | FaceKind::Back => (bmax.x - bmin.x).max(1e-3),
		FaceKind::Left | FaceKind::Right => (bmax.z - bmin.z).max(1e-3),
		_ => 1.0,
	};
	let overlap = aabb_xz_overlap_area(opening, &face_region);
	overlap >= face_len * 0.55 * 0.5
}

fn forward_openings(openings: &Openings) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		if matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Aperture) {
			out.insert(id.clone(), opening.clone());
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId};
	use crate::paneling::clipped_rectangular_strip::ClippedRectangularStripPiece;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn stall_fit_emits_walls_and_interior_labels() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(2.5, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		let (stall, regions) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		// MiniMart / Parts / PublicRestroom leave office/stalls rooms in `within`;
		// other catalog kinds (and Lounge) do not.
		match stall.interior() {
			CommercialStallInterior::MiniMart(_)
			| CommercialStallInterior::Parts(_)
			| CommercialStallInterior::PublicRestroom(_) => {
				assert!(!regions.within.is_empty());
			}
			CommercialStallInterior::Bites(_)
			| CommercialStallInterior::BitesSitdown(_)
			| CommercialStallInterior::KnickKnack(_)
			| CommercialStallInterior::Lounge(_) => {
				assert!(regions.within.is_empty());
			}
		}
		assert_eq!(stall.plan.walls.len(), 4);
		assert!(!stall.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn passage_punches_wall_opening() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(2.5, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		let (stall, _) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		let front = &stall.plan.walls[0];
		let has_clipped = front
			.pieces()
			.iter()
			.any(|p| matches!(p, ClippedRectangularStripPiece::Clipped(_)));
		assert!(has_clipped, "front wall should clip the passage");
		// Solid full-face wall is one panel; a punched door needs jamb/header pieces.
		assert!(
			front.panel_nodes_for_level(LodSceneLevel::High).flatten().len() > 1,
			"door bay should emit framing panels, not a solid sheet"
		);
	}

	#[test]
	fn exclusion_skips_wall_face() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("open_front"),
			Opening::new(
				Aabb3d::from_min_max(Vec3::new(0.0, 0.0, -0.3), Vec3::new(4.0, 3.0, 0.3)),
				OpeningLabel::Exclusion,
			),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0)),
			0.0,
			openings,
		);
		let (stall, _) =
			CommercialStall::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(stall.plan.walls.len() < 4);
	}
}
