//! Storybook Tree canopy as a variable-depth [`BranchOut`] phase machine ([#230](https://github.com/ramate-io/maybraid/issues/230)).
//!
//! Limb hop count is not a free parameter: [`segment_fracs`] only defines length tables for
//! [`STORYBOOK_BRANCH_DEPTH_MIN`]..=[`STORYBOOK_BRANCH_DEPTH_MAX`]. Coerce out-of-range values with
//! [`storybook_branch_depth`] at the anchor/SBS boundary so [`DepthBudget::remaining`] and
//! [`StorybookTreeChain::branch_depth`] stay aligned.

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::{BranchOut, DepthBudget, Hysteresis};

/// Minimum [`StorybookTreeChain::branch_depth`] / [`DepthBudget::remaining`] at a ring seed (RFC).
pub const STORYBOOK_BRANCH_DEPTH_MIN: usize = 3;
/// Maximum supported hops; [`segment_fracs`] has no table beyond this.
pub const STORYBOOK_BRANCH_DEPTH_MAX: usize = 7;

/// Coerce CLI/proto `branch_depth` to a hop count with a matching [`segment_fracs`] row.
pub fn storybook_branch_depth(depth: usize) -> usize {
	depth.clamp(STORYBOOK_BRANCH_DEPTH_MIN, STORYBOOK_BRANCH_DEPTH_MAX)
}

/// RFC segment fractions of total projection length (sum to 1.0) for [`storybook_branch_depth`].
pub fn segment_fracs(depth: usize) -> Vec<f32> {
	match storybook_branch_depth(depth) {
		3 => vec![0.50, 0.30, 0.20],
		4 => vec![0.50, 0.22, 0.18, 0.10],
		5 => vec![0.42, 0.22, 0.18, 0.10, 0.08],
		6 => vec![0.30, 0.20, 0.18, 0.14, 0.10, 0.08],
		_ => vec![0.26, 0.18, 0.16, 0.14, 0.12, 0.08, 0.06],
	}
}

#[derive(Clone)]
pub enum StorybookTreePhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
}

/// One canopy limb (or the stalk): projection budget, distance along limb, and phase state.
#[derive(Clone)]
pub struct StorybookTreeChain {
	pub noise: NoiseConfig,
	/// Total limb length budget from the anchor ring (world units).
	pub projection_length: f32,
	pub branch_depth: usize,
	/// Arc length from the ring anchor along this limb (for RFC outer foliage rule).
	pub distance_from_anchor: f32,
	/// Normalized ring height `0` = lowest, `1` = highest.
	pub ring_u: f32,
	/// Fraction of [`Self::projection_length`] past which outer foliage may appear (RFC `0.65`).
	pub outer_foliage_distance_fraction: f32,
	pub phase: StorybookTreePhase,
}

impl StorybookTreeChain {
	pub fn new(
		noise: NoiseConfig,
		projection_length: f32,
		branch_depth: usize,
		distance_from_anchor: f32,
		ring_u: f32,
		outer_foliage_distance_fraction: f32,
		phase: StorybookTreePhase,
	) -> Self {
		Self {
			noise,
			projection_length,
			branch_depth: storybook_branch_depth(branch_depth),
			distance_from_anchor,
			ring_u,
			outer_foliage_distance_fraction,
			phase,
		}
	}

	fn with_phase(&self, phase: StorybookTreePhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			projection_length: self.projection_length,
			branch_depth: self.branch_depth,
			distance_from_anchor: self.distance_from_anchor,
			ring_u: self.ring_u,
			outer_foliage_distance_fraction: self.outer_foliage_distance_fraction,
		}
	}

	fn with_distance(mut self, distance_from_anchor: f32) -> Self {
		self.distance_from_anchor = distance_from_anchor;
		self
	}

	fn segment_fraction(&self, remaining: usize) -> f32 {
		let fracs = segment_fracs(self.branch_depth);
		let depth = fracs.len();
		let seg_idx = depth.saturating_sub(remaining).min(depth.saturating_sub(1));
		fracs[seg_idx]
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let frac = self.segment_fraction(budget.remaining);
		let len = self.projection_length * frac;
		let lo = len * 0.97;
		let hi = len * 1.03;
		let next_distance = self.distance_from_anchor + len;

		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = lo..hi;

		synced
			.next_hysteresis()
			.into_iter()
			.map(StorybookTreePhase::BranchOut)
			.map(|phase| self.with_phase(phase).with_distance(next_distance))
			.collect()
	}

	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			StorybookTreePhase::BranchOut(b) => Some(&b.inner),
			_ => None,
		}
	}

	/// Hop index from the ring anchor along this limb (`0` at the spoke).
	pub fn branch_order(&self) -> usize {
		match &self.phase {
			StorybookTreePhase::BranchOut(b) => self.branch_depth.saturating_sub(b.remaining),
			StorybookTreePhase::Stalk(_) => 0,
		}
	}
}

/// Highest [`BallStickNode`] on the vertical stalk phase (crown / ring anchor height).
pub fn stalk_tip_from_chain(chain: &crate::BallStickChain<StorybookTreeChain>) -> BallStickNode {
	let mut tip = chain.nodes[0];
	for (node, h) in chain.nodes_with_hysteresis() {
		if matches!(h.phase, StorybookTreePhase::Stalk(_)) && node.position.y >= tip.position.y {
			tip = *node;
		}
	}
	tip
}

/// Whether `node_idx` has no children in a built [`crate::BallStickChain`].
pub fn is_graph_terminal<H: Hysteresis>(chain: &crate::BallStickChain<H>, node_idx: usize) -> bool {
	chain.children.get(node_idx).is_some_and(|c| c.is_empty())
}

impl StorybookTreePhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
		}
	}
}

impl Hysteresis for StorybookTreeChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			StorybookTreePhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(StorybookTreePhase::Stalk(p)))
				.collect(),
			StorybookTreePhase::BranchOut(budget) => self.branch_children(budget),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn storybook_branch_depth_coerces_out_of_range() -> anyhow::Result<()> {
		assert_eq!(storybook_branch_depth(0), STORYBOOK_BRANCH_DEPTH_MIN);
		assert_eq!(storybook_branch_depth(4), 4);
		assert_eq!(storybook_branch_depth(99), STORYBOOK_BRANCH_DEPTH_MAX);
		Ok(())
	}

	#[test]
	fn segment_fracs_sum_to_one() -> anyhow::Result<()> {
		for depth in 3..=STORYBOOK_BRANCH_DEPTH_MAX {
			let fracs = segment_fracs(depth);
			assert_eq!(fracs.len(), depth);
			let sum: f32 = fracs.iter().sum();
			assert!((sum - 1.0).abs() < 1e-5, "depth {depth} sum {sum}");
		}
		Ok(())
	}

	#[test]
	fn limb_reach_near_projection_length() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams { seed: 7, ..Default::default() });
		let fracs = segment_fracs(4);
		let proj = 8.0;
		let first_len = proj * fracs[0];
		let seed = StorybookTreeChain::new(
			noise.clone(),
			proj,
			4,
			0.0,
			0.0,
			0.65,
			StorybookTreePhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::new(0.0, 5.0, 0.0), 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..3)
				.with_length(first_len * 0.97..first_len * 1.03),
				remaining: 4,
			}),
		);
		let root = seed.ball_stick_node().position;
		let chain = crate::BallStickChain::build(vec![seed]);
		let max_dist = chain.nodes.iter().map(|n| n.position.distance(root)).fold(0.0f32, f32::max);
		assert!(max_dist > proj * 0.7, "limb span {max_dist} vs projection {proj}");
		Ok(())
	}

	#[test]
	fn stalk_tip_is_highest_stalk_phase_node() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let seed = StorybookTreeChain::new(
			noise,
			0.0,
			4,
			0.0,
			0.0,
			0.65,
			StorybookTreePhase::Stalk(PointToPoint::new_from_vec3(
				Vec3::ZERO,
				Vec3::new(0.0, 22.0, 0.0),
				0.5,
			)),
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		let tip = stalk_tip_from_chain(&chain);
		assert!((tip.position.y - 22.0).abs() < 1e-3);
		Ok(())
	}
}
