//! **Palm Bush** SBS frontend ([#231](https://github.com/ramate-io/maybraid/issues/231)).

use bevy_math::Vec3;
use procedural_common::NoiseParams;

use crate::anchors::palm_bush::{
	PalmBushProtoAnchors, DEFAULT_CROWN_TUFT_SCALE_FRACTION, DEFAULT_FRONDS_PER_RING,
	DEFAULT_HEIGHT, DEFAULT_RING_COUNT,
};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PalmBushScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_HEIGHT))]
	pub height: f32,
}

impl Default for PalmBushScale {
	fn default() -> Self {
		Self { height: DEFAULT_HEIGHT }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PalmBushCrownParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_RING_COUNT))]
	pub ring_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_FRONDS_PER_RING))]
	pub fronds_per_ring: u32,
}

impl Default for PalmBushCrownParams {
	fn default() -> Self {
		Self { ring_count: DEFAULT_RING_COUNT, fronds_per_ring: DEFAULT_FRONDS_PER_RING }
	}
}

/// Art-directed front-end for Palm Bush.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PalmBushSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: PalmBushScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Crown"))]
	pub crown: PalmBushCrownParams,
	/// Uniform world scale for each [`FrondCrownShape`](crate::FrondCrownShape) ring.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub frond_world_scale: f32,
	/// World scale for the optional concealment tuft at the crown origin (RFC `0.04 * H`).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_CROWN_TUFT_SCALE_FRACTION))]
	pub crown_tuft_scale_factor: f32,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Foliage Noise"))]
	pub foliage_noise: NoiseParams,
}

impl Default for PalmBushSbs {
	fn default() -> Self {
		Self {
			scale: PalmBushScale::default(),
			crown: PalmBushCrownParams::default(),
			frond_world_scale: 1.0,
			crown_tuft_scale_factor: DEFAULT_CROWN_TUFT_SCALE_FRACTION,
			foliage_noise: NoiseParams::default(),
		}
	}
}

impl PalmBushSbs {
	pub fn with_noise_params(mut self, noise: NoiseParams) -> Self {
		self.foliage_noise = noise;
		self
	}

	pub fn height(&self) -> f32 {
		self.scale.height.max(1e-6)
	}

	pub fn with_height(mut self, height: f32) -> Self {
		self.scale.height = height;
		self
	}

	pub fn with_frond_world_scale(mut self, frond_world_scale: f32) -> Self {
		self.frond_world_scale = frond_world_scale;
		self
	}

	pub fn to_proto(&self) -> PalmBushProtoAnchors {
		let defaults = PalmBushProtoAnchors::default();
		PalmBushProtoAnchors {
			height: self.scale.height,
			ring_count: self.crown.ring_count,
			fronds_per_ring: self.crown.fronds_per_ring,
			crown_tuft_scale_fraction: self.crown_tuft_scale_factor,
			..defaults
		}
	}

	pub fn crown_origin(&self) -> Vec3 {
		self.to_proto().crown_origin()
	}

	pub fn crown_ring_position(&self, ring: u32) -> Vec3 {
		self.to_proto().crown_ring_position(ring)
	}

	pub fn crown_tuft_world_scale(&self) -> f32 {
		self.height() * self.crown_tuft_scale_factor
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::palm_bush::{DEFAULT_CROWN_LIFT_FRACTION, DEFAULT_RING_SPACING_FRACTION};
	use anyhow::Result;

	#[test]
	fn crown_origin_at_rfc_lift() -> Result<()> {
		let sbs = PalmBushSbs::default();
		let h = sbs.height();
		let origin = sbs.crown_origin();
		assert!((origin.y - h * DEFAULT_CROWN_LIFT_FRACTION).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn ring_spacing_matches_rfc_fraction() -> Result<()> {
		let sbs = PalmBushSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.ring_spacing() - DEFAULT_RING_SPACING_FRACTION * sbs.height()).abs() < 1e-4);
		Ok(())
	}
}
