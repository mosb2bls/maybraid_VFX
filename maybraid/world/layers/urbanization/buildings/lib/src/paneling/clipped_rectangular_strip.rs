//! Node-chain strip of oriented rectangles with optional per-bay inset openings + crease joints.

use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::rect_crease::joint_along_bay_crease;
use crate::paneling::rect_fit::{OrientedRect, RectInset};
use crate::paneling::rectangle::{ClippedRectangle, Rectangle};
use crate::paneling::rectangular_strip::RectangularStripNode;

/// One bay of a [`ClippedRectangularStrip`].
#[derive(Debug, Clone, PartialEq)]
pub enum ClippedRectangularStripPiece {
	Solid(Rectangle),
	Clipped(ClippedRectangle),
}

impl ClippedRectangularStripPiece {
	pub fn panels(&self) -> Vec<&PanelNode> {
		match self {
			Self::Solid(r) => vec![r.panel_node()],
			Self::Clipped(r) => r.panels().iter().collect(),
		}
	}

	pub fn oriented(&self) -> &OrientedRect {
		match self {
			Self::Solid(r) => &r.oriented,
			Self::Clipped(r) => &r.oriented,
		}
	}

	pub fn thickness(&self) -> f32 {
		match self {
			Self::Solid(r) => r.thickness,
			Self::Clipped(r) => r.thickness,
		}
	}
}

/// Open rectangular strip with optional per-bay [`RectInset`] openings.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRectangularStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	nodes: Vec<RectangularStripNode>,
	pieces: Vec<ClippedRectangularStripPiece>,
}

impl ClippedRectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			nodes: Vec::new(),
			pieces: Vec::new(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	pub fn from_nodes(
		style: PanelStyle,
		nodes: impl IntoIterator<Item = RectangularStripNode>,
		insets: impl IntoIterator<Item = Option<RectInset>>,
	) -> Self {
		let nodes: Vec<RectangularStripNode> = nodes.into_iter().collect();
		let mut insets: Vec<Option<RectInset>> = insets.into_iter().collect();
		if nodes.len() < 2 {
			debug_assert!(false, "ClippedRectangularStrip::from_nodes requires at least 2 nodes");
			return Self::new(style);
		}
		let bay_count = nodes.len() - 1;
		if insets.len() != bay_count {
			debug_assert!(false, "ClippedRectangularStrip::from_nodes insets/bay mismatch");
			insets.resize_with(bay_count, || None);
		}
		let mut strip = Self::new(style);
		strip.nodes = nodes;
		for i in 0..bay_count {
			strip.pieces.push(piece_from_bay(
				style,
				&strip.nodes[i],
				&strip.nodes[i + 1],
				insets[i],
			));
		}
		strip
	}

	pub fn pieces(&self) -> &[ClippedRectangularStripPiece] {
		&self.pieces
	}

	pub fn nodes(&self) -> &[RectangularStripNode] {
		&self.nodes
	}

	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for i in 0..self.pieces.len().saturating_sub(1) {
			let prev = &self.pieces[i];
			let next = &self.pieces[i + 1];
			let thickness = (prev.thickness() + next.thickness()) * 0.5;
			if let Some(j) = joint_along_bay_crease(
				prev.oriented(),
				next.oriented(),
				thickness,
				self.joint_policy,
			) {
				out.push(j);
			}
		}
		out
	}
}

fn piece_from_bay(
	style: PanelStyle,
	a: &RectangularStripNode,
	b: &RectangularStripNode,
	inset: Option<RectInset>,
) -> ClippedRectangularStripPiece {
	let edge = b.position - a.position;
	match inset {
		None => ClippedRectangularStripPiece::Solid(Rectangle::new(
			style,
			a.position,
			edge,
			a.height,
			a.thickness,
			a.roll,
		)),
		Some(inset) => ClippedRectangularStripPiece::Clipped(ClippedRectangle::new(
			style,
			a.position,
			edge,
			a.height,
			a.thickness,
			a.roll,
			inset,
		)),
	}
}

impl BuildingComponents for ClippedRectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			match piece {
				ClippedRectangularStripPiece::Solid(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
				ClippedRectangularStripPiece::Clipped(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
			}
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::from_free(self.joint_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use building_components::panels::PanelGeometry;

	#[test]
	fn middle_inset_splits_pieces() {
		let s = ClippedRectangularStrip::from_nodes(
			PanelStyle::RoughStonework,
			[
				RectangularStripNode::new(Vec3::ZERO, 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 4.0), 2.0, 0.75, 0.0),
			],
			[Some(RectInset::uniform(0.35)), None],
		);
		assert_eq!(s.pieces().len(), 2);
		assert!(matches!(s.pieces()[0], ClippedRectangularStripPiece::Clipped(_)));
		assert!(matches!(s.pieces()[1], ClippedRectangularStripPiece::Solid(_)));
		let clipped = match &s.pieces()[0] {
			ClippedRectangularStripPiece::Clipped(c) => c,
			_ => unreachable!(),
		};
		assert_eq!(clipped.panels().len(), 4);
		assert!(clipped
			.panels()
			.iter()
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}

	#[test]
	fn folded_strip_emits_crease_joint() {
		let s = ClippedRectangularStrip::from_nodes(
			PanelStyle::RoughStonework,
			[
				RectangularStripNode::new(Vec3::ZERO, 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 2.0), 2.5, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 4.0), 2.5, 0.75, 0.0),
			],
			[None, Some(RectInset::uniform(0.35)), None],
		)
		.with_joint_policy(PanelComplexJointPolicy::default());
		assert!(!s.joint_nodes().is_empty());
	}
}
