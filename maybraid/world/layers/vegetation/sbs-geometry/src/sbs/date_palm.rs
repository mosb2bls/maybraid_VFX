//! Restricted **Date Palm** geometry for CLI and playgrounds ([#256](https://github.com/ramate-io/maybraid/issues/256)).

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::anchors::date_palm::{
	DatePalmAnchors, DatePalmProtoAnchors, DEFAULT_STALK_HEIGHT,
	DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::chain::date_palm::DatePalmChain;
use crate::BallStickChain;

/// World scale for Date Palm ([RFC §3.1.7.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/09-date-palm/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct DatePalmScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT))]
	pub stalk_height: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for DatePalmScale {
	fn default() -> Self {
		Self { stalk_height: DEFAULT_STALK_HEIGHT, stalk_base_radius: None }
	}
}

impl DatePalmScale {
	pub fn stalk_base_radius_or_default(&self) -> f32 {
		self.stalk_base_radius
			.unwrap_or(DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT * self.stalk_height)
	}

	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk {
			stalk_height: self.stalk_height,
			stalk_base_radius: self.stalk_base_radius_or_default(),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct DatePalmCrownParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10))]
	pub ring_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 15))]
	pub fronds_per_ring: u32,
}

impl Default for DatePalmCrownParams {
	fn default() -> Self {
		Self { ring_count: 10, fronds_per_ring: 15 }
	}
}

/// Art-directed front-end for Date Palm.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct DatePalmSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: DatePalmScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Crown"))]
	pub crown: DatePalmCrownParams,
	/// Uniform world scale for each [`FrondCrown`] ring at the trunk tip.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.68))]
	pub frond_world_scale: f32,
	/// World scale for the optional concealment tuft at the crown base.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.06))]
	pub crown_tuft_scale_factor: f32,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Trunk Noise"))]
	pub trunk_noise: NoiseParams,
}

impl Default for DatePalmSbs {
	fn default() -> Self {
		Self {
			scale: DatePalmScale::default(),
			crown: DatePalmCrownParams::default(),
			frond_world_scale: 0.68,
			crown_tuft_scale_factor: 0.06,
			trunk_noise: NoiseParams::default(),
		}
	}
}

impl DatePalmSbs {
	pub fn height(&self) -> f32 {
		self.scale.stalk_height.max(1e-6)
	}

	pub fn to_proto(&self) -> DatePalmProtoAnchors {
		let defaults = DatePalmProtoAnchors::default();
		DatePalmProtoAnchors {
			stalk: self.scale.to_stalk(),
			ring_count: self.crown.ring_count,
			fronds_per_ring: self.crown.fronds_per_ring,
			..defaults
		}
	}

	pub fn to_anchors(&self) -> DatePalmAnchors {
		DatePalmAnchors::new(self.to_proto())
	}

	pub fn hysteresis_seeds(&self) -> Vec<DatePalmChain> {
		let noise = NoiseConfig::new(self.trunk_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<DatePalmChain> {
		AnchorsToChain::build_chain(self)
	}

	/// Trunk tip in world space (highest node along +Y from the built chain, or analytic fallback).
	pub fn trunk_tip_from_chain(chain: &BallStickChain<DatePalmChain>) -> Vec3 {
		chain
			.nodes
			.iter()
			.max_by(|a, b| {
				a.position.y.partial_cmp(&b.position.y).unwrap_or(std::cmp::Ordering::Equal)
			})
			.map(|n| n.position)
			.unwrap_or(Vec3::ZERO)
	}

	pub fn trunk_tip(&self) -> Vec3 {
		Self::trunk_tip_from_chain(&self.build_chain())
	}

	/// Crown ring anchor offset from trunk tip (stacked downward along −Y).
	pub fn crown_ring_offset(&self, ring: u32) -> Vec3 {
		-Vec3::Y * self.to_proto().ring_spacing() * ring as f32
	}

	pub fn crown_ring_position(&self, chain: &BallStickChain<DatePalmChain>, ring: u32) -> Vec3 {
		Self::trunk_tip_from_chain(chain) + self.crown_ring_offset(ring)
	}

	pub fn crown_tuft_world_scale(&self) -> f32 {
		self.height() * self.crown_tuft_scale_factor
	}
}

impl Anchors<DatePalmChain> for DatePalmSbs {
	fn anchors(&self) -> Vec<DatePalmChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_stalk_radius_uses_strict_stalk_not_extra_fraction() -> Result<()> {
		let sbs = DatePalmSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.stalk.stalk_base_radius - 0.1 * DEFAULT_STALK_HEIGHT).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn build_chain_reaches_trunk_height() -> Result<()> {
		let sbs = DatePalmSbs::default();
		let proto = sbs.to_proto();
		let chain = sbs.build_chain();
		let tip_y = DatePalmSbs::trunk_tip_from_chain(&chain).y;
		let expected = proto.trunk_height();
		assert!((tip_y - expected).abs() < expected * 0.15, "tip_y {tip_y} vs expected {expected}");
		assert!(chain.nodes.len() >= 6);
		Ok(())
	}

	#[test]
	fn segment_lengths_within_rfc_band() -> Result<()> {
		let sbs = DatePalmSbs::default();
		let h = sbs.height();
		let chain = sbs.build_chain();
		let lo = 0.05 * h;
		let hi = 0.08 * h * 1.05;
		for (seg, _, _) in chain.segments_with_hysteresis() {
			let len = seg.ray().length();
			assert!(len >= lo * 0.9, "segment too short: {len}");
			assert!(len <= hi * 1.1, "segment too long: {len}");
		}
		Ok(())
	}
}
