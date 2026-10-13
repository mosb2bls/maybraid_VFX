//! Sope's Banyan canopy as a **phase machine** on [`super::Hysteresis`] ([#252](https://github.com/ramate-io/maybraid/issues/252)).

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::point_to_point::PointToPoint;
use super::BranchOut;
use super::DepthBudget;
use super::Hysteresis;

/// Authored stalk height the leftover meter hop / radius constants were written against.
pub const AUTHORED_STALK_HEIGHT: f32 = 20.0;

/// Scale a length authored in meters on the 20 m default stalk.
pub fn at_stalk(stalk_height: f32, meters_at_default: f32) -> f32 {
	meters_at_default * (stalk_height / AUTHORED_STALK_HEIGHT).max(1e-6)
}

/// Flair-up segment: one biased [`BranchOut`] step from the current joint.
#[derive(Clone)]
pub struct StartFlairUp {
	pub projection: BranchOut,
}

impl StartFlairUp {
	pub fn sample_from_candidate(
		phase: SopesBanyanPhase,
		_noise: &NoiseConfig,
		stalk_height: f32,
	) -> SopesBanyanPhase {
		match phase {
			SopesBanyanPhase::BranchOut(budget) if budget.remaining < 2 => {
				let inner = &budget.inner;
				let node = inner.node;
				let dof = inner.ray_degrees_of_freedom;
				SopesBanyanPhase::StartFlairUp(StartFlairUp {
					projection: BranchOut::up(node)
						.with_hysteresis_context(
							inner.noise.clone(),
							inner.segment_index,
							inner.incoming_ray,
						)
						.with_ray_degrees_of_freedom(dof * 0.35)
						.with_radius_range(
							at_stalk(stalk_height, 0.11)..at_stalk(stalk_height, 0.12),
						)
						.with_length(at_stalk(stalk_height, 1.0)..at_stalk(stalk_height, 4.0))
						.with_bias_blend(0.7)
						.single_child(),
				})
			}
			other => other,
		}
	}

	pub fn project_to_end(&self) -> SopesBanyanPhase {
		SopesBanyanPhase::EndFlairUp(EndFlairUp { node: self.projection.project_tip() })
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndFlairUp {
	pub node: BallStickNode,
}

/// Descender segment: one downward-biased step.
#[derive(Clone)]
pub struct StartDescender {
	pub projection: BranchOut,
}

impl StartDescender {
	pub fn sample_from_candidate(
		phase: SopesBanyanPhase,
		noise: &NoiseConfig,
		stalk_height: f32,
		banyan_height: f32,
		descender_threshold: f32,
	) -> SopesBanyanPhase {
		match phase {
			SopesBanyanPhase::BranchOut(budget) => {
				let inner = &budget.inner;
				let node = inner.node;
				let sample =
					noise.sample_unit_3d(node.position.x, node.position.y, node.position.z);
				if sample < descender_threshold {
					let drop_len = (banyan_height * 2.0).max(inner.length.end);
					SopesBanyanPhase::StartDescender(StartDescender {
						projection: BranchOut::down(node)
							.with_hysteresis_context(
								inner.noise.clone(),
								inner.segment_index,
								inner.incoming_ray,
							)
							.with_ray_degrees_of_freedom(0.0)
							.with_radius_range(
								at_stalk(stalk_height, 0.10)..at_stalk(stalk_height, 0.2),
							)
							.single_child()
							.with_length(drop_len * 0.92..drop_len * 1.08),
					})
				} else {
					SopesBanyanPhase::BranchOut(budget)
				}
			}
			other => other,
		}
	}

	pub fn project_to_end(&self) -> SopesBanyanPhase {
		SopesBanyanPhase::EndDescender(EndDescender { node: self.projection.project_tip() })
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndDescender {
	pub node: BallStickNode,
}

#[derive(Clone)]
pub enum SopesBanyanPhase {
	Stalk(PointToPoint),
	BranchOut(DepthBudget<BranchOut>),
	StartFlairUp(StartFlairUp),
	EndFlairUp(EndFlairUp),
	StartDescender(StartDescender),
	EndDescender(EndDescender),
}

impl SopesBanyanPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Stalk(p) => &p.start,
			Self::BranchOut(b) => &b.inner.node,
			Self::StartFlairUp(s) => &s.projection.node,
			Self::EndFlairUp(e) => &e.node,
			Self::StartDescender(s) => &s.projection.node,
			Self::EndDescender(e) => &e.node,
		}
	}

	pub fn with_noise(self, noise: NoiseConfig) -> Self {
		match self {
			Self::BranchOut(mut b) => {
				b.inner = b.inner.with_noise(noise);
				Self::BranchOut(b)
			}
			Self::StartFlairUp(mut s) => {
				s.projection = s.projection.with_noise(noise);
				Self::StartFlairUp(s)
			}
			Self::StartDescender(mut s) => {
				s.projection = s.projection.with_noise(noise);
				Self::StartDescender(s)
			}
			other => other,
		}
	}

	/// Flair / descender phase swaps after a mechanical [`DepthBudget`] expansion step.
	pub fn candidate_into(
		self,
		noise: &NoiseConfig,
		stalk_height: f32,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Self {
		match self {
			Self::BranchOut(b) => {
				let p = StartFlairUp::sample_from_candidate(
					Self::BranchOut(b.clone()),
					noise,
					stalk_height,
				);
				StartDescender::sample_from_candidate(
					p,
					noise,
					stalk_height,
					banyan_height,
					descender_threshold,
				)
			}
			other => other,
		}
	}
}

/// One canopy limb: shared noise, vase/descender tuning, and [`SopesBanyanPhase`].
#[derive(Clone)]
pub struct SopesBanyanChain {
	pub noise: NoiseConfig,
	pub stalk_height: f32,
	pub banyan_height: f32,
	pub descender_threshold: f32,
	pub phase: SopesBanyanPhase,
}

impl SopesBanyanChain {
	pub fn new(
		noise: NoiseConfig,
		stalk_height: f32,
		banyan_height: f32,
		descender_threshold: f32,
		phase: SopesBanyanPhase,
	) -> Self {
		Self { noise, stalk_height, banyan_height, descender_threshold, phase }
	}

	fn with_phase(&self, phase: SopesBanyanPhase) -> Self {
		Self {
			phase,
			noise: self.noise.clone(),
			stalk_height: self.stalk_height,
			banyan_height: self.banyan_height,
			descender_threshold: self.descender_threshold,
		}
	}

	fn branch_children(&self, budget: &DepthBudget<BranchOut>) -> Vec<Self> {
		let mut synced = budget.clone();
		synced.inner.noise = self.noise.clone();
		synced
			.next_hysteresis()
			.into_iter()
			.map(SopesBanyanPhase::BranchOut)
			.map(|phase| {
				phase.candidate_into(
					&self.noise,
					self.stalk_height,
					self.banyan_height,
					self.descender_threshold,
				)
			})
			.map(|phase| self.with_phase(phase))
			.collect()
	}

	/// [`BranchOut`] profile for render / tuning (current joint bias when in flair or descender).
	pub fn active_branch_profile(&self) -> Option<&BranchOut> {
		match &self.phase {
			SopesBanyanPhase::BranchOut(b) => Some(&b.inner),
			SopesBanyanPhase::StartFlairUp(s) => Some(&s.projection),
			SopesBanyanPhase::StartDescender(s) => Some(&s.projection),
			_ => None,
		}
	}

	/// Rough segment index along the limb (for coarse render heuristics).
	pub fn segment_depth_hint(&self) -> usize {
		match &self.phase {
			SopesBanyanPhase::Stalk(_p) => 0,
			SopesBanyanPhase::BranchOut(b) => b.inner.segment_index.saturating_add(b.remaining),
			SopesBanyanPhase::StartFlairUp(s) => s.projection.segment_index,
			SopesBanyanPhase::StartDescender(s) => s.projection.segment_index,
			SopesBanyanPhase::EndFlairUp(_) | SopesBanyanPhase::EndDescender(_) => 0,
		}
	}
}

impl Hysteresis for SopesBanyanChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			SopesBanyanPhase::Stalk(p) => p
				.next_hysteresis()
				.into_iter()
				.map(|p| self.with_phase(SopesBanyanPhase::Stalk(p)))
				.collect(),
			SopesBanyanPhase::BranchOut(budget) => self.branch_children(budget),
			SopesBanyanPhase::StartFlairUp(s) => vec![self.with_phase(s.project_to_end())],
			SopesBanyanPhase::EndFlairUp(_) => Vec::new(),
			SopesBanyanPhase::StartDescender(s) => vec![self.with_phase(s.project_to_end())],
			SopesBanyanPhase::EndDescender(_) => Vec::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	#[test]
	fn build_produces_graph() -> anyhow::Result<()> {
		let noise = NoiseConfig::new(NoiseParams::default());
		let seed =
			SopesBanyanChain::new(
				noise.clone(),
				20.0,
				40.0,
				0.12,
				SopesBanyanPhase::BranchOut(DepthBudget {
					inner: BranchOut::up(BallStickNode::new(Vec3::ZERO, 0.05))
						.with_hysteresis_context(noise, 0, Vec3::Y),
					remaining: 3,
				}),
			);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert!(chain.nodes.len() > 1);
		Ok(())
	}
}
