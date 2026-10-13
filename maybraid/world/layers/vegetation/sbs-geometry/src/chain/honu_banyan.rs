//! Honu Banyan canopy as a **phase machine** on [`super::Hysteresis`] ([#250](https://github.com/ramate-io/maybraid/issues/250)).

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::BranchOut;
use super::DepthBudget;
use super::Hysteresis;

/// Descender segment: one downward-biased step (RFC §3.1.6.6).
#[derive(Clone)]
pub struct StartDescender {
	pub projection: BranchOut,
}

impl StartDescender {
	pub fn sample_from_candidate(
		phase: HonuBanyanPhase,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> HonuBanyanPhase {
		match phase {
			HonuBanyanPhase::BranchOut(budget) => {
				let inner = &budget.inner;
				let node = inner.node;
				let sample =
					noise.sample_unit_3d(node.position.x, node.position.y, node.position.z);
				if sample < descender_threshold {
					let drop_len = (banyan_height * 2.0).max(inner.length.end);
					HonuBanyanPhase::StartDescender(StartDescender {
						projection: BranchOut::down(node)
							.with_hysteresis_context(
								inner.noise.clone(),
								inner.segment_index,
								inner.incoming_ray,
							)
							.with_ray_degrees_of_freedom(HONU_DESCENDER_RAY_DOF)
							.with_radius_range(0.08..0.16)
							.single_child()
							.with_length(drop_len * 0.92..drop_len * 1.08),
					})
				} else {
					HonuBanyanPhase::BranchOut(budget)
				}
			}
			other => other,
		}
	}

	pub fn project_to_end(&self) -> HonuBanyanPhase {
		HonuBanyanPhase::EndDescender(EndDescender { node: self.projection.project_tip() })
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndDescender {
	pub node: BallStickNode,
}

#[derive(Clone)]
pub enum HonuBanyanPhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
	StartDescender(StartDescender),
	EndDescender(EndDescender),
}

impl HonuBanyanPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
			Self::StartDescender(s) => &s.projection.node,
			Self::EndDescender(e) => &e.node,
		}
	}

	pub fn is_canopy_limb(&self) -> bool {
		matches!(self, Self::BranchOut(_) | Self::StartDescender(_) | Self::EndDescender(_))
	}

	pub fn is_descender_limb(&self) -> bool {
		matches!(self, Self::StartDescender(_) | Self::EndDescender(_))
	}

	pub fn with_noise(self, noise: NoiseConfig) -> Self {
		match self {
			Self::BranchOut(mut b) => {
				b.inner = b.inner.with_noise(noise);
				Self::BranchOut(b)
			}
			Self::StartDescender(mut s) => {
				s.projection = s.projection.with_noise(noise);
				Self::StartDescender(s)
			}
			other => other,
		}
	}

	pub fn candidate_into(
		self,
		noise: &NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Self {
		match self {
			Self::BranchOut(b) => StartDescender::sample_from_candidate(
				Self::BranchOut(b),
				noise,
				banyan_height,
				descender_threshold,
			),
			other => other,
		}
	}
}

/// One canopy limb: ring metadata, projection budget, and [`HonuBanyanPhase`].
#[derive(Clone)]
pub struct HonuBanyanChain {
	pub noise: NoiseConfig,
	pub tree_height: f32,
	pub ring_u: f32,
	pub projection_length: f32,
	pub branch_depth: usize,
	pub distance_from_anchor: f32,
	pub descender_threshold: f32,
	pub phase: HonuBanyanPhase,
}

impl HonuBanyanChain {
	pub fn new(
		noise: NoiseConfig,
		tree_height: f32,
		ring_u: f32,
		projection_length: f32,
		branch_depth: usize,
		distance_from_anchor: f32,
		descender_threshold: f32,
		phase: HonuBanyanPhase,
	) -> Self {
		Self {
			noise,
			tree_height,
			ring_u,
			projection_length,
			branch_depth: branch_depth.max(1),
			distance_from_anchor,
			descender_threshold,
			phase,
		}
	}

	pub fn height_fraction(&self) -> f32 {
		let y = self.phase.node().position.y;
		(y / self.tree_height.max(1e-6)).clamp(0.0, 1.5)
	}

	fn with_phase(&self, phase: HonuBanyanPhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			tree_height: self.tree_height,
			ring_u: self.ring_u,
			projection_length: self.projection_length,
			branch_depth: self.branch_depth,
			distance_from_anchor: self.distance_from_anchor,
			descender_threshold: self.descender_threshold,
		}
	}

	fn with_distance(mut self, distance_from_anchor: f32) -> Self {
		self.distance_from_anchor = distance_from_anchor;
		self
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let hop = (self.projection_length / self.branch_depth as f32).max(0.1);
		let lo = hop * 0.97;
		let hi = hop * 1.03;
		let next_distance = self.distance_from_anchor + hop;

		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced.inner.length = lo..hi;

		synced
			.next_hysteresis()
			.into_iter()
			.map(HonuBanyanPhase::BranchOut)
			.map(|phase| {
				phase.candidate_into(&self.noise, self.tree_height, self.descender_threshold)
			})
			.map(|phase| self.with_phase(phase).with_distance(next_distance))
			.collect()
	}

	/// Hop index from the ring anchor along this limb (`0` at the spoke).
	pub fn branch_order(&self) -> usize {
		match &self.phase {
			HonuBanyanPhase::BranchOut(b) => self.branch_depth.saturating_sub(b.remaining),
			HonuBanyanPhase::Stalk(_) => 0,
			HonuBanyanPhase::StartDescender(s) => s.projection.segment_index,
			HonuBanyanPhase::EndDescender(_) => 0,
		}
	}

	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			HonuBanyanPhase::BranchOut(b) => Some(&b.inner),
			HonuBanyanPhase::StartDescender(s) => Some(&s.projection),
			_ => None,
		}
	}
}

/// Whether `node_idx` has no children in a built [`crate::BallStickChain`].
pub fn is_graph_terminal(chain: &crate::BallStickChain<HonuBanyanChain>, node_idx: usize) -> bool {
	chain.children.get(node_idx).is_some_and(|c| c.is_empty())
}

impl Hysteresis for HonuBanyanChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			HonuBanyanPhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(HonuBanyanPhase::Stalk(p)))
				.collect(),
			HonuBanyanPhase::BranchOut(budget) => self.branch_children(budget),
			HonuBanyanPhase::StartDescender(s) => vec![self.with_phase(s.project_to_end())],
			HonuBanyanPhase::EndDescender(_) => Vec::new(),
		}
	}
}

/// Canopy limb ray freedom (art-directed; see [`HonuBanyanPhase`] / anchor seeds).
/// Canopy limb ray freedom (art-directed; see [`HonuBanyanPhase`] / anchor seeds).
pub const HONU_CANOPY_ANGLE_TOLERANCE_DEGREES: f32 = 70.0;
pub const HONU_CANOPY_RAY_DOF: f32 = HONU_CANOPY_ANGLE_TOLERANCE_DEGREES.to_radians();

pub const HONU_DESCENDER_ANGLE_TOLERANCE_DEGREES: f32 = 2.0;
pub const HONU_DESCENDER_RAY_DOF: f32 = HONU_DESCENDER_ANGLE_TOLERANCE_DEGREES.to_radians();

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn build_produces_graph() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let seed = HonuBanyanChain::new(
			noise.clone(),
			24.0,
			0.0,
			12.0,
			5,
			0.0,
			0.0,
			HonuBanyanPhase::BranchOut(DepthBudget {
				inner: BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::new(0.0, 20.0, 0.0), 0.05),
					Vec3::X,
				)
				.with_hysteresis_context(noise, 0, Vec3::X),
				remaining: 5,
			}),
		);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert!(chain.nodes.len() > 1);
		Ok(())
	}

	#[test]
	fn phase_is_canopy_limb() -> anyhow::Result<()> {
		assert!(!HonuBanyanPhase::Stalk(PointToPoint::new_from_vec3(Vec3::ZERO, Vec3::Y, 0.5,))
			.is_canopy_limb());
		let noise = NoiseConfig::new(NoiseParams::default());
		assert!(HonuBanyanPhase::BranchOut(DepthBudget {
			inner: BranchOut::radial_out_horizontal(BallStickNode::new(Vec3::ZERO, 0.05), Vec3::X)
				.with_hysteresis_context(noise, 0, Vec3::X),
			remaining: 3,
		})
		.is_canopy_limb());
		Ok(())
	}

	#[test]
	fn zero_descender_threshold_never_starts_descender() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let phase = HonuBanyanPhase::BranchOut(DepthBudget {
			inner: BranchOut::radial_out_horizontal(BallStickNode::new(Vec3::ZERO, 0.05), Vec3::X)
				.with_hysteresis_context(noise.clone(), 3, Vec3::X),
			remaining: 3,
		});
		let out = StartDescender::sample_from_candidate(phase, &noise, 24.0, 0.0);
		assert!(matches!(out, HonuBanyanPhase::BranchOut(_)));
		Ok(())
	}
}
