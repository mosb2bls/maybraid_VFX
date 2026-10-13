//! Stamp Rolling Ground (unchained) — [RFC-105 §3.8.11](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#3811-jersey-rolling-ground-unchained).

use crate::config::JitteredCenter;
use crate::modulation::{RegionAffineModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy)]
pub struct RollingGroundParams {
	/// Number of gentle swell / swale blobs.
	pub count: usize,
	pub size_frac: f32,
	/// Peak |offset|; modulated by [`StampStrength`].
	pub amplitude: f32,
}

impl Default for RollingGroundParams {
	fn default() -> Self {
		Self { count: 4, size_frac: 0.12, amplitude: 3.5 }
	}
}

impl StampStrength for RollingGroundParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.amplitude = scale_additive(self.amplitude, strength);
		self
	}
}

#[derive(Debug, Clone)]
pub struct RollingGround {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: RollingGroundParams,
	pub stamp: StampSet,
}

impl RollingGround {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: RollingGroundParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let amp0 = params.amplitude;
		let radius = short * params.size_frac.clamp(0.05, 0.25);
		let noise = RegionNoise::from_seed(seed.wrapping_add(2), 0.05, radius * 0.1);
		let mut modulations = Vec::new();
		let mut centers = Vec::new();
		let count = params.count.clamp(1, 12);
		for i in 0..count {
			let center = JitteredCenter::default().sample(bounds, seed, 200 + i as u32 * 13);
			centers.push(center);
			let sign = if hash.unit(i as u32 + 3) > 0.45 { 1.0 } else { -1.0 };
			let amp = amp0 * (0.6 + 0.4 * hash.unit(i as u32 + 9)) * sign;
			let region = Region2D::Circle(CircleRegion { center, radius });
			modulations.push(StampModulation::Affine(
				RegionAffineModulation::new(region, 1.0, amp, radius * 0.4, radius * 0.95)
					.with_noise(noise.clone()),
			));
		}

		Self {
			bounds,
			seed,
			params,
			stamp: StampSet {
				modulations,
				spine: centers,
				semantics: StampSemantics::default()
					.with_tag("rolling_ground")
					.with_tag("pasture")
					.with_tag("detail"),
			},
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, RollingGroundParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rolling_emits_modulations() -> anyhow::Result<()> {
		let g = RollingGround::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 300.0, 300.0), 1);
		assert!(!g.stamp.modulations.is_empty());
		assert!(g.stamp.semantics.tags.contains(&"rolling_ground"));
		Ok(())
	}
}
