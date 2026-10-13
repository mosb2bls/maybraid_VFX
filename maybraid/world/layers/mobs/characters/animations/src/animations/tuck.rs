//! Ramp into a held tuck pose via [`FixedTuck`](super::FixedTuck).

mod profile;

pub use profile::TuckProfile;

use crate::animations::FixedTuck;
use crate::Progress;

#[derive(Debug, Clone)]
pub struct Tuck {
	pub fixed: FixedTuck,
}

impl Default for Tuck {
	fn default() -> Self {
		Self { fixed: FixedTuck::default() }
	}
}

impl Tuck {
	pub fn new(tightness: f32) -> Self {
		Self { fixed: FixedTuck::new(tightness) }
	}

	pub fn tightness(&self) -> f32 {
		self.fixed.tightness()
	}

	pub fn profile(&self) -> TuckProfile {
		self.fixed.profile()
	}

	/// Ramp in during the first 15% of progress, then hold through `1.0`.
	pub fn tuck_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < 0.15 {
			t / 0.15
		} else {
			1.0
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tuck_amount_ramps_then_holds() -> anyhow::Result<()> {
		let tuck = Tuck::default();
		assert!(tuck.tuck_amount(0.0).abs() < 1e-5);
		assert!((tuck.tuck_amount(0.15) - 1.0).abs() < 1e-5);
		assert_eq!(tuck.tuck_amount(0.5), 1.0);
		Ok(())
	}
}
