//! Wraps inner [`Hysteresis`] with a **remaining** segment budget for canopy-style growth.

use crate::BallStickNode;

use super::Hysteresis;

/// [`Hysteresis`] plus how many canopy segments remain before flair / terminal transitions.
#[derive(Clone)]
pub struct DepthBudget<H> {
	pub inner: H,
	pub remaining: usize,
}

impl<H: Hysteresis> Hysteresis for DepthBudget<H> {
	fn ball_stick_node(&self) -> BallStickNode {
		self.inner.ball_stick_node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		if self.remaining == 0 {
			return Vec::new();
		}
		self.inner
			.next_hysteresis()
			.into_iter()
			.map(|inner| Self { inner, remaining: self.remaining.saturating_sub(1) })
			.collect()
	}
}
