//! Restricted **Friend's Conifer** geometry for CLI and tree recipes ([#236](https://github.com/ramate-io/maybraid/issues/236), [#238](https://github.com/ramate-io/maybraid/issues/238)).
//!
//! Flattened argument groups map into [`crate::anchors::friends_conifer::FriendsConiferProtoAnchors`].

#[cfg(feature = "clap")]
use procedural_common::noise_params_from_scalar_str;
#[cfg(feature = "clap")]
use procedural_common::parse_unit_range;
use procedural_common::{parse_usize_range, NoiseConfig, NoiseParams, UnitRange, UsizeRange};

use crate::anchors::friends_conifer::FriendsConiferChain;
use crate::anchors::friends_conifer::{
	FriendsConiferAnchorPerturbation, FriendsConiferAnchors, FriendsConiferProtoAnchors,
	FRIENDS_BRANCH_BASE_RADIUS_FRACTION_OF_STALK, FRIENDS_BRANCH_RADIUS_CHILD_SCALE,
	FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT, FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT,
	TEMPERATE_BRANCH_ANGLE_TOLERANCE_RADIANS, TEMPERATE_MAX_PROJECTION_FRACTION_OF_HEIGHT,
	TEMPERATE_MIN_PROJECTION_FRACTION_OF_HEIGHT,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::scale::{stalk_radius_scaled_range, stalk_scaled_range};
use crate::BallStickChain;

/// Full-size Friend's Conifer defaults used to scale mini-tree perturbation and limb caps.
const REFERENCE_STALK_HEIGHT: f32 = 30.0;
const REFERENCE_STALK_BASE_RADIUS: f32 = 0.025 * REFERENCE_STALK_HEIGHT;

/// High-level world scale ([RFC §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FriendsConiferScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 30.0))]
	pub stalk_height: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for FriendsConiferScale {
	fn default() -> Self {
		Self { stalk_height: 30.0, stalk_base_radius: None }
	}
}

impl FriendsConiferScale {
	pub fn stalk_base_radius_or_default(&self) -> f32 {
		self.stalk_base_radius.unwrap_or(0.025 * self.stalk_height)
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
pub struct FriendsRingAnchorParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.10..0.98",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value = "0.04"))]
	pub spacing: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub anchors_per_ring: u32,
}

impl Default for FriendsRingAnchorParams {
	fn default() -> Self {
		Self { height_range: UnitRange::new(0.10, 0.98), spacing: 0.04, anchors_per_ring: 6 }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FriendsLogProjectionParams {
	/// Max and min projection length as fractions of stalk height: `max..min`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.12..0.03",
			value_parser = parse_unit_range,
			value_name = "MAX_FRAC..MIN_FRAC"
		)
	)]
	pub length_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8.0))]
	pub alpha: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3.0))]
	pub beta: f32,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "child-count",
			default_value = "1..2",
			value_parser = parse_usize_range,
			value_name = "MIN..MAX"
		)
	)]
	pub child_count_range: UsizeRange,
}

impl Default for FriendsLogProjectionParams {
	fn default() -> Self {
		Self {
			length_fraction_of_height: UnitRange::new(
				FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT,
				FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT,
			),
			alpha: 8.0,
			beta: 3.0,
			child_count_range: UsizeRange::new(1, 2),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FriendsConiferGrowthParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12.0))]
	pub downward_bias_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 32.0))]
	pub angle_tolerance_degrees: f32,
}

impl Default for FriendsConiferGrowthParams {
	fn default() -> Self {
		Self { branch_depth: 3, downward_bias_degrees: 12.0, angle_tolerance_degrees: 32.0 }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FriendsAnchorPerturbationParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-vertical-perturbation",
			default_value = "-1.0..1.0",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub vertical_offset: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-angular-perturbation",
			default_value = "0.0..2.0",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub angular_scale: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-radius-perturbation",
			default_value = "-0.05..0.05",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub radius_offset: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-perturbation-noise",
			default_value = "1337,0.1,1,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
		)
	)]
	pub noise: NoiseParams,
}

impl Default for FriendsAnchorPerturbationParams {
	fn default() -> Self {
		Self {
			vertical_offset: UnitRange::new(-1.0, 1.0),
			angular_scale: UnitRange::new(0.0, 2.0),
			radius_offset: UnitRange::new(-0.05, 0.05),
			noise: NoiseParams::default(),
		}
	}
}

impl FriendsAnchorPerturbationParams {
	pub fn to_perturbation(&self) -> FriendsConiferAnchorPerturbation {
		FriendsConiferAnchorPerturbation {
			noise: self.noise,
			vertical_offset: self.vertical_offset.start..self.vertical_offset.end,
			angular_scale: self.angular_scale.start..self.angular_scale.end,
			radius_offset: self.radius_offset.start..self.radius_offset.end,
		}
	}
}

/// Art-directed front-end for Friend's Conifer geometry.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FriendsConiferSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: FriendsConiferScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchors"))]
	pub rings: FriendsRingAnchorParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: FriendsLogProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: FriendsConiferGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: FriendsAnchorPerturbationParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
}

impl Default for FriendsConiferSbs {
	fn default() -> Self {
		Self {
			scale: FriendsConiferScale::default(),
			rings: FriendsRingAnchorParams::default(),
			projection: FriendsLogProjectionParams::default(),
			growth: FriendsConiferGrowthParams::default(),
			anchor_perturbation: FriendsAnchorPerturbationParams::default(),
			canopy_noise: NoiseParams::default(),
		}
	}
}

impl FriendsConiferSbs {
	pub fn height(&self) -> f32 {
		self.scale.stalk_height
	}

	/// Shorter limbs and wider ray cone for [Temperate Conifer §3.1.7.15](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/15-temperate-conifer/README.md) ([#238](https://github.com/ramate-io/maybraid/issues/238)).
	pub fn apply_temperate_preset(&mut self) {
		self.projection.length_fraction_of_height = UnitRange::new(
			TEMPERATE_MAX_PROJECTION_FRACTION_OF_HEIGHT,
			TEMPERATE_MIN_PROJECTION_FRACTION_OF_HEIGHT,
		);
		self.growth.angle_tolerance_degrees = TEMPERATE_BRANCH_ANGLE_TOLERANCE_RADIANS.to_degrees();
	}

	pub fn to_anchors(&self) -> FriendsConiferAnchors {
		let mut perturbation = self.anchor_perturbation.to_perturbation();
		let vertical = stalk_scaled_range(
			self.anchor_perturbation.vertical_offset,
			self.scale.stalk_height,
			REFERENCE_STALK_HEIGHT,
		);
		perturbation.vertical_offset = vertical.start..vertical.end;
		let radius = stalk_radius_scaled_range(
			self.anchor_perturbation.radius_offset,
			self.scale.stalk_base_radius_or_default(),
			REFERENCE_STALK_BASE_RADIUS,
		);
		perturbation.radius_offset = radius.start..radius.end;
		FriendsConiferAnchors::new(FriendsConiferProtoAnchors {
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			max_projection_fraction_of_height: self.projection.length_fraction_of_height.start,
			min_projection_fraction_of_height: self.projection.length_fraction_of_height.end,
			projection_alpha: self.projection.alpha,
			projection_beta: self.projection.beta,
			downward_bias_radians: self.growth.downward_bias_degrees.to_radians(),
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			branch_depth: self.growth.branch_depth,
			branch_base_radius_fraction_of_stalk: FRIENDS_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: FRIENDS_BRANCH_RADIUS_CHILD_SCALE,
			child_count_range: self.projection.child_count_range.into(),
			..Default::default()
		})
		.with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<FriendsConiferChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<FriendsConiferChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<FriendsConiferChain> for FriendsConiferSbs {
	fn anchors(&self) -> Vec<FriendsConiferChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = FriendsConiferSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();

		assert_eq!(proto.stalk, sbs.scale.to_stalk());
		assert!(
			(proto.max_projection_fraction_of_height - FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT)
				.abs() < 1e-6
		);
		assert!(
			(proto.min_projection_fraction_of_height - FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT)
				.abs() < 1e-6
		);
		assert_eq!(proto.projection_alpha, 8.0);
		assert_eq!(proto.anchors_per_ring, 6);
		assert_eq!(proto.child_count_range, 1..2);
	}

	#[test]
	fn build_chain_has_stalk_and_branches() -> anyhow::Result<()> {
		let chain = FriendsConiferSbs::default().build_chain();
		assert!(chain.nodes.len() > 8);
		Ok(())
	}

	#[test]
	fn mini_sapling_perturbation_scales_with_stalk() -> Result<()> {
		let mut mini = FriendsConiferSbs::default();
		mini.scale.stalk_height = 3.0;
		let anchors = mini.to_anchors();
		let scale = mini.scale.stalk_height / REFERENCE_STALK_HEIGHT;
		assert!((anchors.perturbation.vertical_offset.start + scale).abs() < 1e-4);
		assert!((anchors.perturbation.vertical_offset.end - scale).abs() < 1e-4);
		assert!(
			anchors.perturbation.radius_offset.end
				<= mini.scale.stalk_base_radius_or_default() * 0.01
		);
		Ok(())
	}
}
