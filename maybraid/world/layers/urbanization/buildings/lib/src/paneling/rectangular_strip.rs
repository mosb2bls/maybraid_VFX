//! Node-chain strip of oriented [`PanelGeometry::Rectangle`] kits + crease joints.
//!
//! Each node owns `(height, thickness, roll)` for its outbound bay. The edge is
//! the vector from this node’s position to the next.

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::rect_crease::joint_along_bay_crease;
use crate::paneling::rectangle::Rectangle;

/// One station along a rectangular strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularStripNode {
	pub position: Vec3,
	pub height: f32,
	pub thickness: f32,
	pub roll: f32,
}

impl RectangularStripNode {
	pub fn new(position: Vec3, height: f32, thickness: f32, roll: f32) -> Self {
		Self { position, height, thickness, roll }
	}
}

/// Open strip; bay `i` uses node `i`’s height / thickness / roll.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	nodes: Vec<RectangularStripNode>,
	bays: Vec<Rectangle>,
}

impl RectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			nodes: Vec::new(),
			bays: Vec::new(),
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
	) -> Self {
		let nodes: Vec<RectangularStripNode> = nodes.into_iter().collect();
		if nodes.len() < 2 {
			debug_assert!(false, "RectangularStrip::from_nodes requires at least 2 nodes");
			return Self::new(style);
		}
		let mut strip = Self::new(style);
		strip.nodes = nodes;
		strip.rebuild_bays();
		strip
	}

	pub fn add_node(&mut self, node: RectangularStripNode) -> &mut Self {
		self.nodes.push(node);
		if self.nodes.len() >= 2 {
			let i = self.nodes.len() - 2;
			self.bays.push(bay_from_nodes(self.style, &self.nodes[i], &self.nodes[i + 1]));
		}
		self
	}

	pub fn bays(&self) -> &[Rectangle] {
		&self.bays
	}

	pub fn nodes(&self) -> &[RectangularStripNode] {
		&self.nodes
	}

	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for i in 0..self.bays.len().saturating_sub(1) {
			let prev = &self.bays[i];
			let next = &self.bays[i + 1];
			let thickness = (prev.thickness + next.thickness) * 0.5;
			if let Some(j) =
				joint_along_bay_crease(&prev.oriented, &next.oriented, thickness, self.joint_policy)
			{
				out.push(j);
			}
		}
		out
	}

	fn rebuild_bays(&mut self) {
		self.bays.clear();
		for w in self.nodes.windows(2) {
			self.bays.push(bay_from_nodes(self.style, &w[0], &w[1]));
		}
	}
}

fn bay_from_nodes(
	style: PanelStyle,
	a: &RectangularStripNode,
	b: &RectangularStripNode,
) -> Rectangle {
	Rectangle::new(style, a.position, b.position - a.position, a.height, a.thickness, a.roll)
}

impl BuildingComponents for RectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for bay in &self.bays {
			out.extend(bay.panel_nodes_for_level(level));
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
	use building_components::panels::PanelGeometry;

	#[test]
	fn three_nodes_two_rectangle_kits() {
		let s = RectangularStrip::from_nodes(
			PanelStyle::RoughStonework,
			[
				RectangularStripNode::new(Vec3::ZERO, 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 4.0), 2.0, 0.75, 0.0),
			],
		);
		assert_eq!(s.bays().len(), 2);
		assert!(s
			.bays()
			.iter()
			.all(|b| matches!(b.panel_node().geometry, PanelGeometry::Rectangle(_))));
		assert_eq!(s.nodes().len(), 3);
		assert!(s.joint_nodes().is_empty());
	}

	#[test]
	fn bay_uses_start_node_height() {
		let s = RectangularStrip::from_nodes(
			PanelStyle::RoughStonework,
			[
				RectangularStripNode::new(Vec3::ZERO, 3.0, 0.5, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 1.0, 0.9, 0.0),
			],
		);
		assert!((s.bays()[0].height - 3.0).abs() < 1e-4);
		assert!((s.bays()[0].thickness - 0.5).abs() < 1e-4);
	}

	#[test]
	fn folded_strip_emits_crease_joint() {
		let s = RectangularStrip::from_nodes(
			PanelStyle::RoughStonework,
			[
				RectangularStripNode::new(Vec3::ZERO, 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(0.0, 0.0, 2.0), 2.0, 0.75, 0.0),
				RectangularStripNode::new(Vec3::new(2.0, 0.0, 2.0), 2.0, 0.75, 0.0),
			],
		);
		assert_eq!(s.joint_nodes().len(), 1);
		let muted = s.clone().with_joint_policy(PanelComplexJointPolicy::never());
		assert!(muted.joint_nodes().is_empty());
	}
}
