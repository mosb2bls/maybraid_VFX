//! Ball-stick **chain** graph: nodes, edges, and pluggable [`Hysteresis`] state per node.

pub mod arch_trunk;
pub mod branch_out;
pub mod child_count;
pub mod date_palm;
pub mod degree_range;
pub mod depth_budget;
pub mod high_bush;
pub mod honu_banyan;
pub mod kamakura_torch;
pub mod length_range;
pub mod liams_conifer;
pub mod penmarch_torch;
pub mod point_to_point;
pub mod radius_range;
pub mod rorys_head_trained;
pub mod sopes_banyan;
pub mod storybook_tree;
pub mod vase_tree;
pub mod waialea_palm;

use std::collections::VecDeque;

use bevy_math::Vec3;

pub use arch_trunk::{
	arch_horizontal_direction_from_yaw_degrees, arch_point, arch_point_from_params,
	normalize_arch_horizontal_direction, ArchTrunk, ArchTrunkParams,
};
pub use branch_out::BranchOut;
pub use depth_budget::DepthBudget;

/// One vertex in the ball-stick graph (position + ball radius at that joint).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BallStickNode {
	pub position: Vec3,
	pub radius: f32,
}

impl BallStickNode {
	pub fn new(position: Vec3, radius: f32) -> Self {
		Self { position, radius }
	}

	/// Horizontal distance from the world \(Y\) axis (trunk axis for upright trees).
	pub fn horizontal_radius(self) -> f32 {
		horizontal_radius_from_y_axis(self.position)
	}
}

/// Horizontal distance from the world \(Y\) axis (trunk axis for upright trees).
pub fn horizontal_radius_from_y_axis(position: Vec3) -> f32 {
	Vec3::new(position.x, 0.0, position.z).length()
}

/// Hysteresis state carried per chain node: minimal surface for non-builder consumers.
///
/// [`BallStickChain::build`] walks [`Self::next_hysteresis`] breadth-first from anchor seeds.
pub trait Hysteresis: Clone {
	/// Geometry for this state (typically a [`BallStickNode`] field on the implementing struct).
	fn ball_stick_node(&self) -> BallStickNode;

	/// Generates the list of states to which this state transitions.
	/// This is the fan out.
	fn next_hysteresis(&self) -> Vec<Self>;
}
#[derive(Debug, Clone)]
pub struct BallStickSegment<'a> {
	pub start: &'a BallStickNode,
	pub end: &'a BallStickNode,
}

impl<'a> BallStickSegment<'a> {
	pub fn ray(&self) -> Vec3 {
		self.end.position - self.start.position
	}

	pub fn midpoint(&self) -> Vec3 {
		(self.start.position + self.end.position) * 0.5
	}

	/// Horizontal distance of the segment midpoint from the world \(Y\) axis.
	pub fn horizontal_radius(&self) -> f32 {
		horizontal_radius_from_y_axis(self.midpoint())
	}
}

#[derive(Clone, Debug)]
pub struct BallStickChain<H>
where
	H: Hysteresis,
{
	pub nodes: Vec<BallStickNode>,
	pub children: Vec<Vec<usize>>,
	pub hysteresis: Vec<H>,
}

impl<H: Hysteresis> Default for BallStickChain<H> {
	fn default() -> Self {
		Self { nodes: Vec::new(), children: Vec::new(), hysteresis: Vec::new() }
	}
}

impl<H: Hysteresis> BallStickChain<H> {
	/// Grow a directed graph from seeds by repeatedly calling [`Hysteresis::next_hysteresis`].
	pub fn build(seeds: Vec<H>) -> Self {
		let mut chain = Self::default();
		let mut queue = VecDeque::new();

		for seed in seeds {
			let idx = chain.push_node(seed.ball_stick_node(), seed);
			queue.push_back(idx);
		}

		while let Some(parent_idx) = queue.pop_front() {
			let parent_state = chain.hysteresis[parent_idx].clone();
			for child_state in parent_state.next_hysteresis() {
				let child_node = child_state.ball_stick_node();
				let child_idx = chain.push_node(child_node, child_state);
				chain.add_child(parent_idx, child_idx);
				queue.push_back(child_idx);
			}
		}

		chain
	}

	fn push_node(&mut self, node: BallStickNode, h: H) -> usize {
		let i = self.nodes.len();
		self.nodes.push(node);
		self.children.push(Vec::new());
		self.hysteresis.push(h);
		i
	}

	fn add_child(&mut self, parent: usize, child: usize) {
		self.children[parent].push(child);
	}

	pub fn nodes(&self) -> impl Iterator<Item = &BallStickNode> {
		self.nodes.iter()
	}

	/// Parallel walk of built geometry and per-node hysteresis (same order as [`Self::nodes`]).
	pub fn nodes_with_hysteresis(&self) -> impl Iterator<Item = (&BallStickNode, &H)> {
		self.nodes.iter().zip(self.hysteresis.iter())
	}

	/// Same as [`Self::nodes_with_hysteresis`], plus each node's graph index (for adjacency via [`Self::children`]).
	pub fn nodes_with_hysteresis_enumerated(
		&self,
	) -> impl Iterator<Item = (usize, &BallStickNode, &H)> {
		self.nodes
			.iter()
			.enumerate()
			.zip(self.hysteresis.iter())
			.map(|((idx, node), h)| (idx, node, h))
	}

	/// Parent node index for `node_idx`, if this node is not a root seed.
	pub fn parent_index(&self, node_idx: usize) -> Option<usize> {
		for (parent, children) in self.children.iter().enumerate() {
			if children.contains(&node_idx) {
				return Some(parent);
			}
		}
		None
	}

	pub fn segments<'a>(&'a self) -> impl Iterator<Item = BallStickSegment<'a>> + 'a {
		self.children.iter().enumerate().flat_map(move |(parent_idx, children)| {
			let start = &self.nodes[parent_idx];
			children
				.iter()
				.map(move |child_idx| BallStickSegment { start, end: &self.nodes[*child_idx] })
		})
	}

	/// Each graph edge with hysteresis at the parent (start) and child (end) nodes.
	pub fn segments_with_hysteresis<'a>(
		&'a self,
	) -> impl Iterator<Item = (BallStickSegment<'a>, &'a H, &'a H)> + 'a {
		self.children.iter().enumerate().flat_map(move |(parent_idx, children)| {
			let start = &self.nodes[parent_idx];
			let parent_h = &self.hysteresis[parent_idx];
			children.iter().map(move |child_idx| {
				let seg = BallStickSegment { start, end: &self.nodes[*child_idx] };
				(seg, parent_h, &self.hysteresis[*child_idx])
			})
		})
	}

	/// Max horizontal distance of any node from the world \(Y\) axis.
	pub fn footprint_radius(&self) -> f32 {
		self.nodes.iter().map(|n| n.horizontal_radius()).fold(0.0_f32, f32::max)
	}

	/// [`Self::footprint_radius`] floored to at least `min_radius`.
	pub fn footprint_radius_at_least(&self, min_radius: f32) -> f32 {
		self.footprint_radius().max(min_radius.max(0.0))
	}
}
