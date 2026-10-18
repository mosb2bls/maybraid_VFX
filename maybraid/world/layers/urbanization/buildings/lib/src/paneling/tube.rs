//! Tube from a polyline of trapezoidal cross-sections.
//!
//! At each node, the inbound/outbound segments define an average perpendicular
//! plane (path pitch/yaw). Authored roll banks the trapezoid in that plane.
//! The four corner polylines become floor, ceiling, left, and right
//! [`ClippedRuledStrip`] faces.

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::path_frame;
pub use path_frame::TubeFrame;

/// One authored station along a [`Tube`] centerline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeCrossSectionNode {
	pub bottom_middle: Vec3,
	pub bottom_left_width: f32,
	pub bottom_right_width: f32,
	pub height: f32,
	pub top_left_width: f32,
	pub top_right_width: f32,
	/// Bank about the average tangent (radians). `0` = unbanked floor in the ⊥ plane.
	pub roll: f32,
	/// When set, top center uses this point instead of `bottom_middle + frame.up * height`.
	///
	/// Lets connector openings carry wall pitch (e.g. a trazaloid face) into the tube
	/// ceiling without forcing the path itself to climb.
	pub top_middle: Option<Vec3>,
}

impl TubeCrossSectionNode {
	pub fn new(
		bottom_middle: Vec3,
		bottom_left_width: f32,
		bottom_right_width: f32,
		height: f32,
		top_left_width: f32,
		top_right_width: f32,
	) -> Self {
		Self {
			bottom_middle,
			bottom_left_width,
			bottom_right_width,
			height,
			top_left_width,
			top_right_width,
			roll: 0.0,
			top_middle: None,
		}
	}

	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	/// Pin the top center in world space (preserves opening pitch under [`corners_at`]).
	pub fn with_top_middle(mut self, top_middle: Vec3) -> Self {
		self.top_middle = Some(top_middle);
		self
	}
}

/// Four corner positions of a cross-section in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeCorners {
	pub bottom_left: Vec3,
	pub bottom_right: Vec3,
	pub top_left: Vec3,
	pub top_right: Vec3,
}

/// Which of the four tube faces are presented via [`BuildingComponents`].
///
/// Geometry for every face is still authored; disabled faces are omitted from
/// panel/joint emission only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TubeFaces {
	pub floor: bool,
	pub ceiling: bool,
	pub left: bool,
	pub right: bool,
}

impl TubeFaces {
	pub const ALL: Self = Self { floor: true, ceiling: true, left: true, right: true };

	pub fn without_floor(mut self) -> Self {
		self.floor = false;
		self
	}

	pub fn without_ceiling(mut self) -> Self {
		self.ceiling = false;
		self
	}

	pub fn without_left(mut self) -> Self {
		self.left = false;
		self
	}

	pub fn without_right(mut self) -> Self {
		self.right = false;
		self
	}
}

impl Default for TubeFaces {
	fn default() -> Self {
		Self::ALL
	}
}

/// Polyline tube: four [`ClippedRuledStrip`] faces from trapezoid stations.
#[derive(Debug, Clone, PartialEq)]
pub struct Tube {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	faces: TubeFaces,
	nodes: Vec<TubeCrossSectionNode>,
	floor: ClippedRuledStrip,
	ceiling: ClippedRuledStrip,
	left: ClippedRuledStrip,
	right: ClippedRuledStrip,
}

impl Tube {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			faces: TubeFaces::ALL,
			nodes: Vec::new(),
			floor: ClippedRuledStrip::new(style),
			ceiling: ClippedRuledStrip::new(style),
			left: ClippedRuledStrip::new(style),
			right: ClippedRuledStrip::new(style),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	/// Bulk construct with solid faces (no bay clips).
	pub fn from_nodes(
		style: PanelStyle,
		nodes: impl IntoIterator<Item = TubeCrossSectionNode>,
	) -> Self {
		Self::from_nodes_with_clips(
			style,
			nodes,
			std::iter::empty(),
			std::iter::empty(),
			std::iter::empty(),
			std::iter::empty(),
		)
	}

	/// Bulk construct. Per-face clip lists should match bay count (`nodes - 1`);
	/// empty or mismatched lengths pad/truncate with `debug_assert` (same as strips).
	pub fn from_nodes_with_clips(
		style: PanelStyle,
		nodes: impl IntoIterator<Item = TubeCrossSectionNode>,
		floor_clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
		ceiling_clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
		left_clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
		right_clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
	) -> Self {
		let nodes: Vec<TubeCrossSectionNode> = nodes.into_iter().collect();
		if nodes.len() < 2 {
			debug_assert!(
				false,
				"Tube::from_nodes requires at least 2 stations (got {})",
				nodes.len()
			);
			return Self::new(style);
		}

		let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
		let mut bottom_left = Vec::with_capacity(nodes.len());
		let mut bottom_right = Vec::with_capacity(nodes.len());
		let mut top_left = Vec::with_capacity(nodes.len());
		let mut top_right = Vec::with_capacity(nodes.len());
		for i in 0..nodes.len() {
			let c = corners_at_framed(&nodes, &positions, i);
			bottom_left.push(c.bottom_left);
			bottom_right.push(c.bottom_right);
			top_left.push(c.top_left);
			top_right.push(c.top_right);
		}

		let bay_count = nodes.len() - 1;
		let floor_clips = normalize_clips(floor_clips, bay_count, "floor");
		let ceiling_clips = normalize_clips(ceiling_clips, bay_count, "ceiling");
		let left_clips = normalize_clips(left_clips, bay_count, "left");
		let right_clips = normalize_clips(right_clips, bay_count, "right");

		let floor = ClippedRuledStrip::from_lines(
			style,
			bottom_left.clone(),
			bottom_right.clone(),
			floor_clips,
		);
		let ceiling = ClippedRuledStrip::from_lines(
			style,
			top_left.clone(),
			top_right.clone(),
			ceiling_clips,
		);
		let left =
			ClippedRuledStrip::from_lines(style, bottom_left.clone(), top_left.clone(), left_clips);
		let right = ClippedRuledStrip::from_lines(style, bottom_right, top_right, right_clips);

		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			faces: TubeFaces::ALL,
			nodes,
			floor,
			ceiling,
			left,
			right,
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.floor = std::mem::replace(&mut self.floor, ClippedRuledStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self.ceiling = std::mem::replace(&mut self.ceiling, ClippedRuledStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self.left = std::mem::replace(&mut self.left, ClippedRuledStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self.right = std::mem::replace(&mut self.right, ClippedRuledStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self
	}

	/// Select which faces emit panels/joints. Defaults to [`TubeFaces::ALL`].
	pub fn with_faces(mut self, faces: TubeFaces) -> Self {
		self.faces = faces;
		self
	}

	pub fn faces(&self) -> TubeFaces {
		self.faces
	}

	pub fn nodes(&self) -> &[TubeCrossSectionNode] {
		&self.nodes
	}

	pub fn floor(&self) -> &ClippedRuledStrip {
		&self.floor
	}

	pub fn ceiling(&self) -> &ClippedRuledStrip {
		&self.ceiling
	}

	pub fn left(&self) -> &ClippedRuledStrip {
		&self.left
	}

	pub fn right(&self) -> &ClippedRuledStrip {
		&self.right
	}

	/// Corner rails for every station (same order as [`Self::nodes`]).
	pub fn corners(&self) -> Vec<TubeCorners> {
		let positions: Vec<Vec3> = self.nodes.iter().map(|n| n.bottom_middle).collect();
		(0..self.nodes.len())
			.map(|i| corners_at_framed(&self.nodes, &positions, i))
			.collect()
	}

	pub fn frame_at(&self, index: usize) -> Option<TubeFrame> {
		if index >= self.nodes.len() {
			return None;
		}
		Some(frame_at(&self.nodes, index))
	}
}

impl BuildingComponents for Tube {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if self.faces.floor {
			out.extend(self.floor.panel_nodes_for_level(level));
		}
		if self.faces.ceiling {
			out.extend(self.ceiling.panel_nodes_for_level(level));
		}
		if self.faces.left {
			out.extend(self.left.panel_nodes_for_level(level));
		}
		if self.faces.right {
			out.extend(self.right.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		if self.faces.floor {
			out.extend(self.floor.joint_nodes_for_level(level));
		}
		if self.faces.ceiling {
			out.extend(self.ceiling.joint_nodes_for_level(level));
		}
		if self.faces.left {
			out.extend(self.left.joint_nodes_for_level(level));
		}
		if self.faces.right {
			out.extend(self.right.joint_nodes_for_level(level));
		}
		out
	}
}

fn normalize_clips(
	clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
	bay_count: usize,
	face: &str,
) -> Vec<Option<Vec<Vec3>>> {
	let mut clips: Vec<Option<Vec<Vec3>>> = clips.into_iter().collect();
	if clips.is_empty() {
		return vec![None; bay_count];
	}
	if clips.len() != bay_count {
		debug_assert!(
			false,
			"Tube {} clips.len()={} != bay_count={}",
			face,
			clips.len(),
			bay_count
		);
		clips.resize_with(bay_count, || None);
	}
	clips
}

/// Average tangent and zero-roll basis, then apply node roll about the tangent.
pub fn frame_at(nodes: &[TubeCrossSectionNode], index: usize) -> TubeFrame {
	let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
	path_frame::path_frame(&positions, index, nodes[index].roll)
}

pub fn corners_at(nodes: &[TubeCrossSectionNode], index: usize) -> TubeCorners {
	let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
	corners_at_framed(nodes, &positions, index)
}

fn corners_at_framed(
	nodes: &[TubeCrossSectionNode],
	positions: &[Vec3],
	index: usize,
) -> TubeCorners {
	let node = &nodes[index];
	let frame = path_frame::path_frame(positions, index, node.roll);
	let top_middle = node.top_middle.unwrap_or_else(|| node.bottom_middle + frame.up * node.height);
	TubeCorners {
		bottom_left: node.bottom_middle - frame.right * node.bottom_left_width,
		bottom_right: node.bottom_middle + frame.right * node.bottom_right_width,
		top_left: top_middle - frame.right * node.top_left_width,
		top_right: top_middle + frame.right * node.top_right_width,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::paneling::clipped_ruled_strip::ClippedStripPiece;

	fn approx_eq(a: Vec3, b: Vec3) -> bool {
		(a - b).length() < 1e-4
	}

	fn level_node(z: f32, half_w: f32, height: f32) -> TubeCrossSectionNode {
		TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, z), half_w, half_w, height, half_w, half_w)
	}

	#[test]
	fn level_straight_cardinal_corners() {
		let nodes =
			vec![level_node(0.0, 1.0, 2.0), level_node(2.0, 1.0, 2.0), level_node(4.0, 1.0, 2.0)];
		let tube = Tube::from_nodes(PanelStyle::RoughStonework, nodes);
		assert_eq!(tube.nodes().len(), 3);

		let c0 = tube.corners()[0];
		assert!(approx_eq(c0.bottom_left, Vec3::new(-1.0, 0.0, 0.0)));
		assert!(approx_eq(c0.bottom_right, Vec3::new(1.0, 0.0, 0.0)));
		assert!(approx_eq(c0.top_left, Vec3::new(-1.0, 2.0, 0.0)));
		assert!(approx_eq(c0.top_right, Vec3::new(1.0, 2.0, 0.0)));

		assert_eq!(tube.floor().pieces().len(), 1);
		assert!(matches!(tube.floor().pieces()[0], ClippedStripPiece::Solid(_)));
		// 2 bays × 2 triangles
		assert_eq!(tube.floor().pieces()[0].as_complex().triangles().len(), 4);
		assert_eq!(tube.ceiling().pieces().len(), 1);
		assert_eq!(tube.left().pieces().len(), 1);
		assert_eq!(tube.right().pieces().len(), 1);
	}

	#[test]
	fn pitched_path_tilts_up_with_average_tangent() {
		// Rise along +Z: middle station should have up tilted in the YZ plane.
		let nodes = vec![
			TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0, 1.0, 1.0),
			TubeCrossSectionNode::new(Vec3::new(0.0, 1.0, 2.0), 1.0, 1.0, 1.0, 1.0, 1.0),
			TubeCrossSectionNode::new(Vec3::new(0.0, 2.0, 4.0), 1.0, 1.0, 1.0, 1.0, 1.0),
		];
		let frame = frame_at(&nodes, 1);
		assert!(frame.up.z.abs() > 0.1, "up should tilt with pitch, got {:?}", frame.up);
		assert!(frame.up.x.abs() < 1e-4, "pitch in YZ should keep up.x ~ 0, got {:?}", frame.up);
		assert!(
			approx_eq(frame.right, Vec3::X) || approx_eq(frame.right, Vec3::new(1.0, 0.0, 0.0))
		);
		// right stays horizontal for a pure YZ pitch
		assert!(frame.right.y.abs() < 1e-4);
	}

	#[test]
	fn authored_top_middle_preserves_pitch_offset() {
		let nodes = vec![
			TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, 2.0, 1.0, 1.0)
				.with_top_middle(Vec3::new(0.5, 2.0, 0.0)),
			TubeCrossSectionNode::new(Vec3::new(0.0, 0.0, 2.0), 1.0, 1.0, 2.0, 1.0, 1.0)
				.with_top_middle(Vec3::new(0.5, 2.0, 2.0)),
		];
		let c0 = corners_at(&nodes, 0);
		assert!(approx_eq(c0.top_left, Vec3::new(-0.5, 2.0, 0.0)));
		assert!(approx_eq(c0.top_right, Vec3::new(1.5, 2.0, 0.0)));
	}

	#[test]
	fn non_zero_roll_banks_in_perp_plane() {
		let nodes = vec![
			level_node(0.0, 1.0, 2.0).with_roll(0.0),
			level_node(2.0, 1.0, 2.0).with_roll(std::f32::consts::FRAC_PI_2),
			level_node(4.0, 1.0, 2.0).with_roll(0.0),
		];
		let frame = frame_at(&nodes, 1);
		// +90° about +Z: right₀=X → Y, up₀=Y → -X
		assert!(approx_eq(frame.right, Vec3::Y));
		assert!(approx_eq(frame.up, Vec3::new(-1.0, 0.0, 0.0)));

		let c = corners_at(&nodes, 1);
		assert!(approx_eq(c.bottom_left, Vec3::new(0.0, -1.0, 2.0)));
		assert!(approx_eq(c.bottom_right, Vec3::new(0.0, 1.0, 2.0)));
	}

	#[test]
	#[cfg(not(debug_assertions))]
	fn short_input_yields_empty() {
		let tube = Tube::from_nodes(PanelStyle::RoughStonework, [level_node(0.0, 1.0, 1.0)]);
		assert!(tube.nodes().is_empty());
		assert!(tube.floor().pieces().is_empty());
	}

	#[test]
	#[cfg(debug_assertions)]
	#[should_panic(expected = "at least 2 stations")]
	fn short_input_debug_asserts() {
		let _ = Tube::from_nodes(PanelStyle::RoughStonework, [level_node(0.0, 1.0, 1.0)]);
	}

	#[test]
	fn two_stations_minimum_builds_faces() {
		let tube = Tube::from_nodes(
			PanelStyle::RoughStonework,
			[level_node(0.0, 1.0, 1.0), level_node(2.0, 1.0, 1.0)],
		);
		assert_eq!(tube.nodes().len(), 2);
		assert_eq!(tube.floor().pieces().len(), 1);
		assert_eq!(tube.floor().pieces()[0].as_complex().triangles().len(), 2);
	}

	#[test]
	fn disabled_faces_omit_presentation() {
		use building_components::BuildingComponents;
		use lod::scene::LodSceneLevel;

		let nodes = [level_node(0.0, 1.0, 1.0), level_node(2.0, 1.0, 1.0)];
		let full = Tube::from_nodes(PanelStyle::RoughStonework, nodes);
		let open = full.clone().with_faces(TubeFaces::ALL.without_ceiling().without_floor());
		assert!(open.faces().floor == false && open.faces().ceiling == false);
		assert!(open.faces().left && open.faces().right);
		// Strips still authored.
		assert!(!open.floor().pieces().is_empty());
		assert!(!open.ceiling().pieces().is_empty());

		let full_n = full.panel_nodes_for_level(LodSceneLevel::High).len();
		let open_n = open.panel_nodes_for_level(LodSceneLevel::High).len();
		assert!(open_n < full_n);
		assert!(open_n > 0);
	}
}
