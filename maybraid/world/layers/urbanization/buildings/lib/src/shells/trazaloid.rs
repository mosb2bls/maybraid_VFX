//! A trapezoid-like shell: two stacked frustum bands with an optional waist reveal.
//!
//! Good for modern and sci-fi buildings, outposts, and the like.
//!
//! Lower band: four walls. Upper band: four walls. Optional footprint floor and
//! ridge ceiling ([`TrazaloidSlab`]: [`None`](TrazaloidSlab::None) /
//! [`Solid`](TrazaloidSlab::Solid)). Passages map to centered lower-band doors;
//! apertures are not mapped (the waist gap is the window). Slab-cutting openings
//! may dig a centered hole in Solid slabs or remove them.

mod geometry;
mod openings;
mod slabs;

#[cfg(test)]
mod tests;

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use building_components::joints::geometry::JointPost;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;

use crate::openings::{MappedOpenings, Openings};
use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::panel_complex::{PanelComplexJointPolicy, DEFAULT_PANEL_THICKNESS};

pub use geometry::TrazaloidSide;
use geometry::{face_bottom_pair, PlanRect, PostSegment};

/// Horizontal footprint / ridge slab presentation for towering ownership.
///
/// Openings with [`crate::openings::OpeningLabel::cuts_slab`] may still cut a
/// Solid slab (centered hole from intersection scale, or full removal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrazaloidSlab {
	/// Omit the slab entirely.
	None,
	/// Solid rectangular strip; openings may dig a hole or remove it.
	Solid,
}

impl Default for TrazaloidSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for a [`Trazaloid`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct TrazaloidParams {
	/// Footprint center; `y` is the base elevation. Openings are local to this origin.
	pub origin: Vec3,
	/// Full width (X) / depth (Z) at the footprint (`y = 0` in local space).
	pub footprint: Vec2,
	/// Full width (X) / depth (Z) at the ridge.
	pub ridge: Vec2,
	pub lower_height: f32,
	pub upper_height: f32,
	/// Vertical gap between lower top and upper bottom (the “window” band).
	pub band_vertical_offset: f32,
	/// Inward meters from the linear footprint→ridge silhouette at lower-top height.
	pub waist_horizontal_offset: f32,
	/// Local-space void plan applied at construct time (then translated by [`Self::origin`]).
	///
	/// **Walls:** `Passage` openings are assigned to the nearest lower-band side;
	/// the largest face-aligned extent on each side wins and becomes a centered
	/// ground door (width / height from that AABB). `Aperture` is ignored for
	/// wall mapping — use [`Self::band_vertical_offset`] as the window reveal.
	///
	/// **Slabs:** only [`OpeningLabel::cuts_slab`](crate::openings::OpeningLabel::cuts_slab)
	/// labels affect Solid floor / ceiling (intersection scale → centered hole,
	/// or remove the slab when scale ≥ the smaller plan side).
	pub openings: Openings,
	/// Footprint floor at `y = 0` (default: absent).
	pub floor: TrazaloidSlab,
	/// Ridge ceiling (default: solid).
	pub ceiling: TrazaloidSlab,
	pub style: PanelStyle,
	pub joint_thickness: f32,
	/// Extra vertical posts per face (between corners) at [`LodSceneLevel::High`].
	pub face_post_count: u32,
}

impl Default for TrazaloidParams {
	fn default() -> Self {
		let footprint = Vec2::new(8.0, 6.0);
		Self {
			origin: Vec3::ZERO,
			footprint,
			ridge: Vec2::new(4.0, 3.0),
			lower_height: 3.0,
			upper_height: 2.5,
			band_vertical_offset: 0.35,
			waist_horizontal_offset: 0.25,
			openings: Openings::new(),
			floor: TrazaloidSlab::None,
			ceiling: TrazaloidSlab::Solid,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
			face_post_count: 2,
		}
	}
}

impl TrazaloidParams {
	pub fn new(footprint: Vec2, ridge: Vec2, lower_height: f32, upper_height: f32) -> Self {
		Self { footprint, ridge, lower_height, upper_height, ..Self::default() }
	}

	pub fn origin(mut self, origin: Vec3) -> Self {
		self.origin = origin;
		self
	}

	pub fn band_vertical_offset(mut self, gap: f32) -> Self {
		self.band_vertical_offset = gap;
		self
	}

	pub fn waist_horizontal_offset(mut self, inset: f32) -> Self {
		self.waist_horizontal_offset = inset;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	pub fn floor(mut self, floor: TrazaloidSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: TrazaloidSlab) -> Self {
		self.ceiling = ceiling;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn face_post_count(mut self, count: u32) -> Self {
		self.face_post_count = count;
		self
	}

	pub fn build(self) -> Trazaloid {
		Trazaloid::new(self)
	}
}

/// Two-band trapezoidal-pyramid shell.
#[derive(Debug, Clone, PartialEq)]
pub struct Trazaloid {
	params: TrazaloidParams,
	joint_policy: PanelComplexJointPolicy,
	lower_walls: [ClippedRuledStrip; 4],
	upper_walls: [ClippedRuledStrip; 4],
	floor: Option<ClippedRuledStrip>,
	ceiling: Option<ClippedRuledStrip>,
	high_posts: Vec<PostSegment>,
	/// Resolved plan rectangles (foot, waist/lower-top, upper-bottom, ridge).
	rects: [PlanRect; 4],
	/// Winning passage openings (at most one per side).
	openings: Openings,
	/// Contact geometry for those passages.
	mapped: MappedOpenings,
}

fn offset_openings(openings: &Openings, delta: Vec3) -> Openings {
	if delta == Vec3::ZERO {
		return openings.clone();
	}
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		let min = Vec3::from(opening.bounds.min) + delta;
		let max = Vec3::from(opening.bounds.max) + delta;
		out.insert(
			id.clone(),
			crate::openings::Opening::new(Aabb3d::from_min_max(min, max), opening.label.clone()),
		);
	}
	out
}

impl Trazaloid {
	pub fn new(params: TrazaloidParams) -> Self {
		let mut params = params;
		params.openings = offset_openings(&params.openings, params.origin);
		let rects = params.resolve_rects();
		let [foot, waist, upper_bot, ridge] = rects;
		let style = params.style;
		let policy = PanelComplexJointPolicy::default();

		let side_passages = params.resolve_side_passages(foot, waist);
		let mut openings = Openings::new();
		let mut mapped = MappedOpenings::new();
		let mut side_clip: [Option<Vec<Vec3>>; 4] = [None, None, None, None];
		for (i, passage) in side_passages.into_iter().enumerate() {
			let Some(p) = passage else {
				continue;
			};
			side_clip[i] = Some(p.clip);
			mapped.insert(p.id.clone(), p.mapped);
			openings.insert(p.id, p.opening);
		}

		let lower_walls = TrazaloidSide::all().map(|side| {
			let (a0, b0) = face_bottom_pair(side, foot);
			let (a1, b1) = face_bottom_pair(side, waist);
			let clip = side_clip[side as usize].clone();
			ClippedRuledStrip::from_lines(style, [a0, a1], [b0, b1], [clip])
				.with_joint_policy(policy)
		});

		let upper_walls = TrazaloidSide::all().map(|side| {
			let (a0, b0) = face_bottom_pair(side, upper_bot);
			let (a1, b1) = face_bottom_pair(side, ridge);
			ClippedRuledStrip::from_lines(style, [a0, a1], [b0, b1], [None])
				.with_joint_policy(policy)
		});

		let floor = params.resolve_floor_slab(style, policy, foot);
		let ceiling = params.resolve_ceiling_slab(style, policy, ridge);
		let high_posts = params.build_high_posts(&rects);

		Self {
			params,
			joint_policy: policy,
			lower_walls,
			upper_walls,
			floor,
			ceiling,
			high_posts,
			rects,
			openings,
			mapped,
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.lower_walls = self.lower_walls.map(|w| w.with_joint_policy(joint_policy));
		self.upper_walls = self.upper_walls.map(|w| w.with_joint_policy(joint_policy));
		if let Some(floor) = self.floor.take() {
			self.floor = Some(floor.with_joint_policy(joint_policy));
		}
		if let Some(ceiling) = self.ceiling.take() {
			self.ceiling = Some(ceiling.with_joint_policy(joint_policy));
		}
		self
	}

	pub fn params(&self) -> &TrazaloidParams {
		&self.params
	}

	pub fn lower_walls(&self) -> &[ClippedRuledStrip; 4] {
		&self.lower_walls
	}

	pub fn upper_walls(&self) -> &[ClippedRuledStrip; 4] {
		&self.upper_walls
	}

	pub fn floor(&self) -> Option<&ClippedRuledStrip> {
		self.floor.as_ref()
	}

	pub fn ceiling(&self) -> Option<&ClippedRuledStrip> {
		self.ceiling.as_ref()
	}

	/// Foot, waist, upper-bottom, ridge full extents and heights.
	pub fn plan_levels(&self) -> [(f32, Vec2); 4] {
		self.rects.map(|r| (r.y, Vec2::new(r.half_x * 2.0, r.half_z * 2.0)))
	}
}

impl BuildingComponents for Trazaloid {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.lower_walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		for w in &self.upper_walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		if let Some(floor) = &self.floor {
			out.extend(floor.panel_nodes_for_level(level));
		}
		if let Some(ceiling) = &self.ceiling {
			out.extend(ceiling.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for w in &self.lower_walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		for w in &self.upper_walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		if let Some(floor) = &self.floor {
			out.extend(floor.joint_nodes_for_level(level));
		}
		if let Some(ceiling) = &self.ceiling {
			out.extend(ceiling.joint_nodes_for_level(level));
		}

		if matches!(level, LodSceneLevel::High) {
			let thickness = self.params.joint_thickness.max(1e-4);
			for seg in &self.high_posts {
				if let Some(placement) =
					JointPost::placed_along_crease(seg.start, seg.end, thickness, seg.radial_hint)
				{
					out.push_free(JointNode::rough_stone_post(placement));
				}
			}
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let p = self.params();
		let height = p.lower_height + p.band_vertical_offset + p.upper_height;
		Some(BuildingStructuralLodProbe::from_volumes([MassingVolume::trazaloid(
			p.origin,
			p.footprint,
			p.ridge,
			height,
		)]))
	}
}
