//! Liam's Conifer canopy as a **fixed-depth** [`BranchOut`] phase machine ([#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! # Phases
//!
//! - [`LiamsConiferPhase::Stalk`] — vertical trunk segment(s) via [`super::point_to_point::PointToPoint`].
//! - [`LiamsConiferPhase::BranchOut`] — sparse canopy limb with [`super::DepthBudget`].
//!
//! # Segment lengths
//!
//! Each limb carries a world [`LiamsConiferChain::projection_length`] from its anchor ring.
//! On each hop, [`Self::branch_children`] sets [`BranchOut::length`] to
//! `projection_length * SEGMENT_FRACS[segment_index]` before calling [`DepthBudget::next_hysteresis`].
//!
//! # Segment radii
//!
//! Anchor seeds set a thick base radius and collapsed [`BranchOut::radius_range`]; thinning is
//! **not** reconfigured here — it follows [`BranchOut::radius_range_child_scale`] in
//! [`super::branch_out::BranchOut::expand_children`].

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::{BranchOut, DepthBudget, Hysteresis};

/// RFC segment fractions of total projection length (sum to 1.0).
pub const SEGMENT_FRACS: [f32; 3] = [0.70, 0.15, 0.15];

/// Coerce proto/SBS `branch_depth` to `1..=[`SEGMENT_FRACS`].len()` (RFC default `3`).
pub fn liams_conifer_branch_depth(depth: usize) -> usize {
	depth.clamp(1, SEGMENT_FRACS.len())
}

#[derive(Clone)]
pub enum LiamsConiferPhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
}

/// One canopy limb (or the stalk): shared noise, projection budget, and phase state.
#[derive(Clone)]
pub struct LiamsConiferChain {
	pub noise: NoiseConfig,
	/// Total limb length budget from the anchor ring (world units).
	pub projection_length: f32,
	/// Hops remaining; should match [`SEGMENT_FRACS`].len() at the ring seed.
	pub branch_depth: usize,
	pub phase: LiamsConiferPhase,
}

impl LiamsConiferChain {
	pub fn new(
		noise: NoiseConfig,
		projection_length: f32,
		branch_depth: usize,
		phase: LiamsConiferPhase,
	) -> Self {
		Self { noise, projection_length, branch_depth, phase }
	}

	fn with_phase(&self, phase: LiamsConiferPhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			projection_length: self.projection_length,
			branch_depth: self.branch_depth,
		}
	}

	/// Maps [`DepthBudget::remaining`] to a [`SEGMENT_FRACS`] entry (`remaining == depth` → first segment).
	fn segment_fraction(&self, remaining: usize) -> f32 {
		let depth = liams_conifer_branch_depth(self.branch_depth);
		let seg_idx = depth.saturating_sub(remaining).min(SEGMENT_FRACS.len() - 1);
		SEGMENT_FRACS[seg_idx]
	}

	/// Apply this hop's length fraction, then fan out via [`DepthBudget`].
	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let frac = self.segment_fraction(budget.remaining);
		let len = self.projection_length * frac;
		let lo = len * 0.97;
		let hi = len * 1.03;

		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = lo..hi;

		synced
			.next_hysteresis()
			.into_iter()
			.map(LiamsConiferPhase::BranchOut)
			.map(|phase| self.with_phase(phase))
			.collect()
	}

	/// [`BranchOut`] profile for anchor perturbation and render heuristics.
	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			LiamsConiferPhase::BranchOut(b) => Some(&b.inner),
			_ => None,
		}
	}
}

impl LiamsConiferPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
		}
	}

	pub fn with_noise(self, noise: NoiseConfig) -> Self {
		match self {
			Self::BranchOut(mut b) => {
				b.inner = b.inner.with_noise(noise);
				Self::BranchOut(b)
			}
			other => other,
		}
	}
}

impl Hysteresis for LiamsConiferChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			LiamsConiferPhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(LiamsConiferPhase::Stalk(p)))
				.collect(),
			LiamsConiferPhase::BranchOut(budget) => self.branch_children(budget),
		}
	}
}

/// Highest [`BallStickNode`] on the vertical stalk phase (tree crown).
pub fn stalk_tip_from_chain(
	chain: &crate::BallStickChain<LiamsConiferChain>,
) -> crate::BallStickNode {
	let mut tip = chain.nodes[0];
	for (node, h) in chain.nodes_with_hysteresis() {
		if matches!(h.phase, LiamsConiferPhase::Stalk(_)) && node.position.y >= tip.position.y {
			tip = *node;
		}
	}
	tip
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn limb_reach_near_projection_length() -> anyhow::Result<()> {
		use crate::BallStickChain;

		let noise = NoiseConfig::new(NoiseParams { seed: 7, ..Default::default() });
		let seed = LiamsConiferChain::new(
			noise.clone(),
			4.5,
			3,
			LiamsConiferPhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::new(0.0, 3.0, 0.0), 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..2)
				.single_child(),
				remaining: 3,
			}),
		);
		let root = seed.ball_stick_node().position;
		let chain = BallStickChain::build(vec![seed]);
		let max_dist = chain.nodes.iter().map(|n| n.position.distance(root)).fold(0.0f32, f32::max);
		assert!(
			max_dist > 4.5 * 0.85,
			"limb span {max_dist} should approach projection length 4.5"
		);
		Ok(())
	}

	#[test]
	fn build_produces_sparse_graph() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let seed = LiamsConiferChain::new(
			noise.clone(),
			1.5,
			3,
			LiamsConiferPhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::ZERO, 0.04),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X)
				.with_child_count(1..2)
				.single_child(),
				remaining: 3,
			}),
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert!(chain.nodes.len() > 1);
		Ok(())
	}

	#[test]
	fn stalk_tip_is_highest_stalk_phase_node() -> anyhow::Result<()> {
		let seed = LiamsConiferChain::new(
			NoiseConfig::new(NoiseParams::default()),
			0.0,
			3,
			LiamsConiferPhase::Stalk(PointToPoint::new_from_vec3(
				Vec3::ZERO,
				Vec3::new(0.0, 30.0, 0.0),
				0.5,
			)),
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		let tip = stalk_tip_from_chain(&chain);
		assert!((tip.position.y - 30.0).abs() < 1e-3);
		Ok(())
	}
}
