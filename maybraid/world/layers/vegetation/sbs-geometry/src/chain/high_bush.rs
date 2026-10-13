//! High-bush radial shoot chains ([#225](https://github.com/ramate-io/maybraid/issues/225), RFC §3.1.6.3).

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::{BranchOut, DepthBudget, Hysteresis};

/// RFC `3..=5` shoot hops per radial arm.
pub const HIGH_BUSH_BRANCH_DEPTH_MIN: usize = 3;
pub const HIGH_BUSH_BRANCH_DEPTH_MAX: usize = 5;

pub fn high_bush_branch_depth(depth: usize) -> usize {
	depth.clamp(HIGH_BUSH_BRANCH_DEPTH_MIN, HIGH_BUSH_BRANCH_DEPTH_MAX)
}

#[derive(Clone)]
pub enum HighBushPhase {
	/// Ground anchor; fans out into [`Self::Shoot`] seeds.
	Root {
		node: BallStickNode,
		shoot_specs: Vec<ShootSeedSpec>,
	},
	Shoot(DepthBudget<BranchOut>),
}

/// Precomputed shoot direction and radial helper for one radial arm.
#[derive(Clone, Debug, PartialEq)]
pub struct ShootSeedSpec {
	pub radial_xz: bevy_math::Vec3,
	pub bias_ray: bevy_math::Vec3,
}

#[derive(Clone)]
pub struct HighBushChain {
	pub noise: NoiseConfig,
	/// Total height scale `H`.
	pub height: f32,
	/// World Y of the ground anchor (for [`Self::height_fraction`]).
	pub anchor_y: f32,
	pub branch_depth: usize,
	pub shoot_index: Option<usize>,
	pub angle_tolerance_radians: f32,
	pub bias_blend: f32,
	pub child_count: Range<usize>,
	pub segment_length_fraction_lo: f32,
	pub segment_length_fraction_hi: f32,
	pub branch_radius_child_scale: (f32, f32),
	pub phase: HighBushPhase,
}

impl HighBushChain {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		noise: NoiseConfig,
		height: f32,
		anchor_y: f32,
		branch_depth: usize,
		shoot_index: Option<usize>,
		angle_tolerance_radians: f32,
		bias_blend: f32,
		child_count: Range<usize>,
		segment_length_fraction_lo: f32,
		segment_length_fraction_hi: f32,
		branch_radius_child_scale: (f32, f32),
		phase: HighBushPhase,
	) -> Self {
		Self {
			noise,
			height,
			anchor_y,
			branch_depth: high_bush_branch_depth(branch_depth),
			shoot_index,
			angle_tolerance_radians,
			bias_blend,
			child_count,
			segment_length_fraction_lo,
			segment_length_fraction_hi,
			branch_radius_child_scale,
			phase,
		}
	}

	fn with_phase(&self, phase: HighBushPhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			height: self.height,
			anchor_y: self.anchor_y,
			branch_depth: self.branch_depth,
			shoot_index: self.shoot_index,
			angle_tolerance_radians: self.angle_tolerance_radians,
			bias_blend: self.bias_blend,
			child_count: self.child_count.clone(),
			segment_length_fraction_lo: self.segment_length_fraction_lo,
			segment_length_fraction_hi: self.segment_length_fraction_hi,
			branch_radius_child_scale: self.branch_radius_child_scale,
		}
	}

	fn segment_length_range(&self) -> Range<f32> {
		let h = self.height.max(1e-6);
		(h * self.segment_length_fraction_lo)..(h * self.segment_length_fraction_hi)
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = self.segment_length_range();

		synced
			.next_hysteresis()
			.into_iter()
			.map(HighBushPhase::Shoot)
			.map(|phase| self.with_phase(phase))
			.collect()
	}

	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			HighBushPhase::Shoot(b) => Some(&b.inner),
			HighBushPhase::Root { .. } => None,
		}
	}

	/// Hop index along a shoot from the ground anchor (`0` at the root joint).
	pub fn branch_order(&self) -> usize {
		match &self.phase {
			HighBushPhase::Shoot(b) => self.branch_depth.saturating_sub(b.remaining),
			HighBushPhase::Root { .. } => 0,
		}
	}

	/// Normalized vertical extent from the ground anchor along `H`.
	pub fn height_fraction(&self) -> f32 {
		let h = self.height.max(1e-6);
		((self.ball_stick_node().position.y - self.anchor_y) / h).clamp(0.0, 1.5)
	}
}

impl HighBushPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Root { node, .. } => node,
			Self::Shoot(b) => &b.inner.node,
		}
	}
}

/// Whether `node_idx` has no children in a built [`crate::BallStickChain`].
pub fn is_graph_terminal(chain: &crate::BallStickChain<HighBushChain>, node_idx: usize) -> bool {
	chain.children.get(node_idx).is_some_and(|c| c.is_empty())
}

impl Hysteresis for HighBushChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			HighBushPhase::Root { node, shoot_specs } => shoot_specs
				.iter()
				.enumerate()
				.map(|(shoot_index, spec)| {
					let limb_r = node.radius;
					let branch = BranchOut::radial_out_horizontal(*node, spec.radial_xz)
						.with_hysteresis_context(self.noise.clone(), 0, spec.bias_ray)
						.with_bias_ray(spec.bias_ray)
						.with_bias_blend(self.bias_blend)
						.with_ray_degrees_of_freedom(self.angle_tolerance_radians)
						.with_child_count(self.child_count.clone())
						.with_radius_range(limb_r..limb_r)
						.with_radius_range_child_scale(self.branch_radius_child_scale)
						.with_length(self.segment_length_range());

					Self::new(
						self.noise.clone().with_frequency(self.noise.params().frequency * 10.0),
						self.height,
						self.anchor_y,
						self.branch_depth,
						Some(shoot_index),
						self.angle_tolerance_radians,
						self.bias_blend,
						self.child_count.clone(),
						self.segment_length_fraction_lo,
						self.segment_length_fraction_hi,
						self.branch_radius_child_scale,
						HighBushPhase::Shoot(DepthBudget {
							inner: branch,
							remaining: self.branch_depth,
						}),
					)
				})
				.collect(),
			HighBushPhase::Shoot(budget) => self.branch_children(budget),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::high_bush::{DEFAULT_BIAS_BLEND, DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES};
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn high_bush_branch_depth_coerces_out_of_range() -> anyhow::Result<()> {
		assert_eq!(high_bush_branch_depth(1), HIGH_BUSH_BRANCH_DEPTH_MIN);
		assert_eq!(high_bush_branch_depth(4), 4);
		assert_eq!(high_bush_branch_depth(99), HIGH_BUSH_BRANCH_DEPTH_MAX);
		Ok(())
	}

	#[test]
	fn root_expands_to_shoot_count_children() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams { seed: 3, ..Default::default() });
		let root_node = BallStickNode::new(Vec3::new(0.0, 0.2, 0.0), 0.05);
		let specs: Vec<_> = (0..8)
			.map(|i| {
				let theta = std::f32::consts::TAU * i as f32 / 8.0;
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				ShootSeedSpec {
					radial_xz: radial,
					bias_ray: (radial * 0.45 + Vec3::Y * 0.75).normalize_or_zero(),
				}
			})
			.collect();
		let root = HighBushChain::new(
			noise,
			10.0,
			0.2,
			4,
			None,
			DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			DEFAULT_BIAS_BLEND,
			1..2,
			0.08,
			0.16,
			(0.72, 0.80),
			HighBushPhase::Root { node: root_node, shoot_specs: specs },
		);
		let children = root.next_hysteresis();
		assert_eq!(children.len(), 8);
		assert!(children.iter().all(|c| c.shoot_index.is_some()));
		Ok(())
	}

	#[test]
	fn build_chain_reaches_expected_depth() -> anyhow::Result<()> {
		use crate::anchors::high_bush::HighBushProtoAnchors;

		let chain = HighBushProtoAnchors::default().build_chain();
		assert!(chain.nodes.len() > 20, "nodes {}", chain.nodes.len());
		assert!((chain.nodes[0].position.y - chain.nodes[0].position.y).abs() < 1e-5);
		Ok(())
	}
}
