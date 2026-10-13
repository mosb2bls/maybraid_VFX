//! Date Palm vertical trunk as [`DepthBudget`] + [`BranchOut`] ([#256](https://github.com/ramate-io/maybraid/issues/256)).

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::BranchOut;
use super::DepthBudget;
use super::Hysteresis;

/// Trunk-only growth phase (crown fronds spawn outside the graph).
#[derive(Clone)]
pub enum DatePalmPhase {
	Trunk(DepthBudget<BranchOut>),
}

/// One Date Palm instance: tight vertical trunk chain.
#[derive(Clone)]
pub struct DatePalmChain {
	pub noise: NoiseConfig,
	pub phase: DatePalmPhase,
}

impl DatePalmChain {
	pub fn new(noise: NoiseConfig, phase: DatePalmPhase) -> Self {
		Self { noise, phase }
	}

	fn with_phase(&self, phase: DatePalmPhase) -> Self {
		Self { noise: self.noise.clone(), phase }
	}
}

impl DatePalmPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Trunk(b) => &b.inner.node,
		}
	}
}

impl Hysteresis for DatePalmChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			DatePalmPhase::Trunk(budget) => budget
				.next_hysteresis()
				.into_iter()
				.map(|phase| self.with_phase(DatePalmPhase::Trunk(phase)))
				.collect(),
		}
	}
}
