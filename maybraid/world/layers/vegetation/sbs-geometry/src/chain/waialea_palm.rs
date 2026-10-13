//! Waialea Palm arched trunk as [`DepthBudget`] + [`super::ArchTrunk`] ([#255](https://github.com/ramate-io/maybraid/issues/255)).

use procedural_common::NoiseConfig;

use crate::BallStickNode;

use super::ArchTrunk;
use super::DepthBudget;
use super::Hysteresis;

/// Trunk-only growth phase (crown fronds spawn outside the graph).
#[derive(Clone)]
pub enum WaialeaPalmPhase {
	Trunk(DepthBudget<ArchTrunk>),
}

/// One Waialea Palm instance: arched trunk chain.
#[derive(Clone)]
pub struct WaialeaPalmChain {
	pub noise: NoiseConfig,
	pub phase: WaialeaPalmPhase,
}

impl WaialeaPalmChain {
	pub fn new(noise: NoiseConfig, phase: WaialeaPalmPhase) -> Self {
		Self { noise, phase }
	}

	fn with_phase(&self, phase: WaialeaPalmPhase) -> Self {
		Self { noise: self.noise.clone(), phase }
	}
}

impl WaialeaPalmPhase {
	pub fn node(&self) -> &BallStickNode {
		match self {
			Self::Trunk(b) => &b.inner.node,
		}
	}
}

impl Hysteresis for WaialeaPalmChain {
	fn ball_stick_node(&self) -> BallStickNode {
		*self.phase.node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		match &self.phase {
			WaialeaPalmPhase::Trunk(budget) => budget
				.next_hysteresis()
				.into_iter()
				.map(|phase| self.with_phase(WaialeaPalmPhase::Trunk(phase)))
				.collect(),
		}
	}
}
