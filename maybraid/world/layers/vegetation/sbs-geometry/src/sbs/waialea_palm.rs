//! Restricted **Waialea Palm** geometry for CLI and playgrounds ([#255](https://github.com/ramate-io/maybraid/issues/255)).

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::waialea_palm::{
	WaialeaPalmAnchors, WaialeaPalmProtoAnchors, DEFAULT_ARCH_LATERAL_FRACTION,
	DEFAULT_ARCH_YAW_DEGREES, DEFAULT_STALK_HEIGHT, DEFAULT_TRUNK_HEIGHT_FRACTION,
	DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::anchors::{Anchors, AnchorsToChain};
use crate::chain::waialea_palm::WaialeaPalmChain;
use crate::BallStickChain;

/// World scale for Waialea Palm ([RFC §3.1.7.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct WaialeaPalmScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT))]
	pub stalk_height: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for WaialeaPalmScale {
	fn default() -> Self {
		Self { stalk_height: DEFAULT_STALK_HEIGHT, stalk_base_radius: None }
	}
}

impl WaialeaPalmScale {
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
pub struct WaialeaPalmCrownParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3))]
	pub ring_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10))]
	pub fronds_per_ring: u32,
}

impl Default for WaialeaPalmCrownParams {
	fn default() -> Self {
		Self { ring_count: 3, fronds_per_ring: 10 }
	}
}

/// Trunk height and arch controls for playground iteration.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct WaialeaPalmTrunkParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TRUNK_HEIGHT_FRACTION))]
	pub trunk_height_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_ARCH_LATERAL_FRACTION))]
	pub arch_lateral_fraction: f32,
	/// Horizontal lean direction as yaw about world +Y in degrees (`0` → +X).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_ARCH_YAW_DEGREES))]
	pub arch_yaw_degrees: f32,
}

impl Default for WaialeaPalmTrunkParams {
	fn default() -> Self {
		Self {
			trunk_height_fraction: DEFAULT_TRUNK_HEIGHT_FRACTION,
			arch_lateral_fraction: DEFAULT_ARCH_LATERAL_FRACTION,
			arch_yaw_degrees: DEFAULT_ARCH_YAW_DEGREES,
		}
	}
}

/// Art-directed front-end for Waialea Palm.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct WaialeaPalmSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: WaialeaPalmScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Trunk"))]
	pub trunk: WaialeaPalmTrunkParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Crown"))]
	pub crown: WaialeaPalmCrownParams,
	/// Uniform world scale for each [`FrondCrown`] ring at the trunk tip.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub frond_world_scale: f32,
	/// World scale for the optional concealment tuft at the crown base.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.04))]
	pub crown_tuft_scale_factor: f32,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Trunk Noise"))]
	pub trunk_noise: NoiseParams,
}

impl Default for WaialeaPalmSbs {
	fn default() -> Self {
		Self {
			scale: WaialeaPalmScale::default(),
			trunk: WaialeaPalmTrunkParams::default(),
			crown: WaialeaPalmCrownParams::default(),
			frond_world_scale: 0.55,
			crown_tuft_scale_factor: 0.04,
			trunk_noise: NoiseParams::default(),
		}
	}
}

impl WaialeaPalmSbs {
	pub fn height(&self) -> f32 {
		self.scale.stalk_height.max(1e-6)
	}

	pub fn to_proto(&self) -> WaialeaPalmProtoAnchors {
		let defaults = WaialeaPalmProtoAnchors::default();
		WaialeaPalmProtoAnchors {
			stalk: self.scale.to_stalk(),
			trunk_height_fraction: self.trunk.trunk_height_fraction,
			arch_lateral_fraction: self.trunk.arch_lateral_fraction,
			arch_yaw_degrees: self.trunk.arch_yaw_degrees,
			ring_count: self.crown.ring_count,
			fronds_per_ring: self.crown.fronds_per_ring,
			..defaults
		}
	}

	pub fn to_anchors(&self) -> WaialeaPalmAnchors {
		WaialeaPalmAnchors::new(self.to_proto())
	}

	pub fn hysteresis_seeds(&self) -> Vec<WaialeaPalmChain> {
		let noise = NoiseConfig::new(self.trunk_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<WaialeaPalmChain> {
		AnchorsToChain::build_chain(self)
	}

	/// Trunk tip in world space (highest node along +Y from the built chain).
	pub fn trunk_tip_from_chain(chain: &BallStickChain<WaialeaPalmChain>) -> Vec3 {
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

	/// Direction of the final trunk segment (unit); falls back to +Y.
	pub fn trunk_tip_tangent_from_chain(chain: &BallStickChain<WaialeaPalmChain>) -> Vec3 {
		let mut ray = Vec3::Y;
		for (seg, _, _) in chain.segments_with_hysteresis() {
			let r = seg.ray();
			if r.length_squared() > 1e-8 {
				ray = r.normalize();
			}
		}
		ray
	}

	/// Blend of world up and tip tangent so crown rings follow a gentle arch without lying flat.
	pub fn crown_stack_direction(chain: &BallStickChain<WaialeaPalmChain>) -> Vec3 {
		const UP_BIAS: f32 = 0.55;
		let tangent = Self::trunk_tip_tangent_from_chain(chain);
		let blended = Vec3::Y * UP_BIAS + tangent * (1.0 - UP_BIAS);
		if blended.length_squared() < 1e-8 {
			Vec3::Y
		} else {
			blended.normalize()
		}
	}

	/// Crown ring anchor offset from trunk tip (stacked along [`Self::crown_stack_direction`]).
	pub fn crown_ring_offset(&self, ring: u32, chain: &BallStickChain<WaialeaPalmChain>) -> Vec3 {
		Self::crown_stack_direction(chain) * self.to_proto().ring_spacing() * ring as f32
	}

	pub fn crown_ring_position(&self, chain: &BallStickChain<WaialeaPalmChain>, ring: u32) -> Vec3 {
		Self::trunk_tip_from_chain(chain) + self.crown_ring_offset(ring, chain)
	}

	pub fn crown_tuft_world_scale(&self) -> f32 {
		self.height() * self.crown_tuft_scale_factor
	}
}

impl Anchors<WaialeaPalmChain> for WaialeaPalmSbs {
	fn anchors(&self) -> Vec<WaialeaPalmChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_stalk_radius_matches_rfc_fraction() -> Result<()> {
		let sbs = WaialeaPalmSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.stalk.stalk_base_radius - 0.025 * DEFAULT_STALK_HEIGHT).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn build_chain_reaches_trunk_height_and_arch() -> Result<()> {
		let sbs = WaialeaPalmSbs::default();
		let proto = sbs.to_proto();
		let chain = sbs.build_chain();
		let tip = WaialeaPalmSbs::trunk_tip_from_chain(&chain);
		let expected_y = proto.trunk_height();
		assert!((tip.y - expected_y).abs() < expected_y * 0.12, "tip_y {} vs {expected_y}", tip.y);
		let dir = proto.arch_direction();
		let expected_lean = proto.arch_lateral_fraction * expected_y;
		let lateral = tip - Vec3::Y * tip.y;
		assert!(
			(lateral.length() - expected_lean).abs() < expected_y * 0.15,
			"tip lateral {} vs {expected_lean}",
			lateral.length()
		);
		assert!(lateral.dot(dir) > 0.0, "lean should follow arch direction");
		assert!(chain.nodes.len() >= 8);
		Ok(())
	}

	#[test]
	fn sbs_trunk_params_flow_to_proto() -> Result<()> {
		let mut sbs = WaialeaPalmSbs::default();
		sbs.trunk.arch_lateral_fraction = 0.2;
		sbs.trunk.arch_yaw_degrees = 90.0;
		sbs.trunk.trunk_height_fraction = 0.75;
		let proto = sbs.to_proto();
		assert!((proto.arch_lateral_fraction - 0.2).abs() < 1e-5);
		assert!((proto.arch_yaw_degrees - 90.0).abs() < 1e-5);
		assert!((proto.trunk_height_fraction - 0.75).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn mid_chain_segment_leans_along_arch() -> Result<()> {
		let sbs = WaialeaPalmSbs::default();
		let chain = sbs.build_chain();
		let dir = sbs.to_proto().arch_direction();
		let mid_idx = chain.nodes.len() / 2;
		let mid = chain.nodes[mid_idx].position;
		let along = Vec3::new(mid.x, 0.0, mid.z).dot(dir);
		assert!(along > 0.05, "mid-chain should lean along arch, got {mid:?}");
		Ok(())
	}

	#[test]
	fn segment_lengths_within_rfc_band() -> Result<()> {
		let sbs = WaialeaPalmSbs::default();
		let h = sbs.height();
		let chain = sbs.build_chain();
		let lo = 0.05 * h;
		let hi = 0.08 * h * 1.05;
		for (seg, _, _) in chain.segments_with_hysteresis() {
			let len = seg.ray().length();
			assert!(len >= lo * 0.85, "segment too short: {len}");
			assert!(len <= hi * 1.15, "segment too long: {len}");
		}
		Ok(())
	}

	#[test]
	fn ring_spacing_matches_rfc_fraction() -> Result<()> {
		let sbs = WaialeaPalmSbs::default();
		assert!((sbs.to_proto().ring_spacing() - 0.015 * sbs.height()).abs() < 1e-4);
		Ok(())
	}
}
