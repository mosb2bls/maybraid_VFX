//! Restricted **Liam's Conifer** geometry for CLI and playgrounds ([#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! Flattened argument groups map into [`crate::anchors::liams_conifer::LiamsConiferProtoAnchors`], then
//! [`AnchorsToChain::build_chain`] grows the shared ball-stick graph.

#[cfg(feature = "clap")]
use procedural_common::noise_params_from_scalar_str;
#[cfg(feature = "clap")]
use procedural_common::parse_unit_range;
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use crate::anchors::liams_conifer::{
	LiamsConiferAnchorPerturbation, LiamsConiferAnchors, LiamsConiferProtoAnchors,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::scale::{stalk_radius_scaled_range, stalk_scaled_range};
use crate::{BallStickChain, LiamsConiferChain};

/// Full-size Liam's Conifer defaults used to scale mini-tree perturbation and limb caps.
const REFERENCE_STALK_HEIGHT: f32 = 30.0;
const REFERENCE_STALK_BASE_RADIUS: f32 = 0.025 * REFERENCE_STALK_HEIGHT;

/// High-level world scale for Liam's Conifer ([RFC §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md)).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct LiamsConiferScale {
	/// Height of the strict vertical stalk in world units.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 30.0))]
	pub stalk_height: f32,
	/// Radius of the stalk base (`0.025 * H` when unset via [`Self::stalk_base_radius_or_default`]).
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for LiamsConiferScale {
	fn default() -> Self {
		Self { stalk_height: 30.0, stalk_base_radius: None }
	}
}

impl LiamsConiferScale {
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
pub struct RingAnchorParams {
	/// First and last ring heights as fractions of stalk height: `first..last`.
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
	/// Vertical ring spacing as a fraction of stalk height.
	#[cfg_attr(feature = "clap", arg(long, default_value = "0.03"))]
	pub spacing: f32,
	/// Anchors placed per ring (RFC ~4; denser default ~6, every 60°).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub anchors_per_ring: u32,
}

impl Default for RingAnchorParams {
	fn default() -> Self {
		Self { height_range: UnitRange::new(0.10, 0.98), spacing: 0.03, anchors_per_ring: 6 }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct ConiferProjectionParams {
	/// Max projection length as a fraction of stalk height, with optional min floor as `max..min_fraction_of_max`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.20..0.30",
			value_parser = parse_unit_range,
			value_name = "MAX_FRAC..MIN_FRAC_OF_MAX"
		)
	)]
	pub length_fraction_of_height: UnitRange,
}

impl Default for ConiferProjectionParams {
	fn default() -> Self {
		Self { length_fraction_of_height: UnitRange::new(0.20, 0.30) }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct ConiferGrowthParams {
	/// Limb hops (`1`–`3`); RFC table is [`SEGMENT_FRACS`](crate::chain::liams_conifer::SEGMENT_FRACS) (default `3`, coerced at anchor build).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3))]
	pub branch_depth: usize,
	/// Downward radial bias in degrees.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 2.0))]
	pub downward_bias_degrees: f32,
	/// Ray perturbation tolerance in degrees (RFC ~8°).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 27.0))]
	pub angle_tolerance_degrees: f32,
}

impl Default for ConiferGrowthParams {
	fn default() -> Self {
		Self { branch_depth: 3, downward_bias_degrees: 2.0, angle_tolerance_degrees: 27.0 }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct AnchorPerturbationParams {
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
			default_value = "0.0..0.5",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub angular_scale: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-radius-perturbation",
			default_value = "-0.5..0.5",
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

impl Default for AnchorPerturbationParams {
	fn default() -> Self {
		Self {
			vertical_offset: UnitRange::new(-1.0, 1.0),
			angular_scale: UnitRange::new(0.0, 0.5),
			radius_offset: UnitRange::new(-0.5, 0.5),
			noise: NoiseParams::default().with_frequency(0.1),
		}
	}
}

impl AnchorPerturbationParams {
	pub fn to_perturbation(&self) -> LiamsConiferAnchorPerturbation {
		LiamsConiferAnchorPerturbation {
			noise: self.noise,
			vertical_offset: self.vertical_offset.start..self.vertical_offset.end,
			angular_scale: self.angular_scale.start..self.angular_scale.end,
			radius_offset: self.radius_offset.start..self.radius_offset.end,
		}
	}
}

/// Art-directed front-end for Liam's Conifer.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct LiamsConiferSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: LiamsConiferScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchors"))]
	pub rings: RingAnchorParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: ConiferProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: ConiferGrowthParams,
	/// World tuft scale numerator (`tuft_scale_factor * H`); foliage rendering lands in `chico-sbs-trees`.
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = 0.02, help_heading = "Terminal foliage")
	)]
	pub tuft_scale_factor: f32,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: AnchorPerturbationParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
}

impl Default for LiamsConiferSbs {
	fn default() -> Self {
		Self {
			scale: LiamsConiferScale::default(),
			rings: RingAnchorParams::default(),
			projection: ConiferProjectionParams::default(),
			growth: ConiferGrowthParams::default(),
			tuft_scale_factor: 0.02,
			anchor_perturbation: AnchorPerturbationParams::default(),
			canopy_noise: NoiseParams::default(),
		}
	}
}

impl LiamsConiferSbs {
	/// Suggested world-space tuft scale (`tuft_scale_factor × stalk height`).
	pub fn tuft_world_scale(&self) -> f32 {
		self.scale.stalk_height * self.tuft_scale_factor
	}

	/// Anchor recipe used by [`Self::build_chain`] / [`Anchors::anchors`].
	pub fn to_anchors(&self) -> LiamsConiferAnchors {
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
		LiamsConiferAnchors::new(LiamsConiferProtoAnchors {
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			max_projection_fraction_of_height: self.projection.length_fraction_of_height.start,
			min_projection_fraction_of_max: self.projection.length_fraction_of_height.end,
			downward_bias_radians: self.growth.downward_bias_degrees.to_radians(),
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			branch_depth: self.growth.branch_depth,
			// Limb thickness: base radius at ring + down-step only (see proto docs).
			branch_base_radius_fraction_of_stalk: 0.1,
			branch_radius_child_scale: (0.72, 0.80),
			linear_projection_taper: false,
		})
		.with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<LiamsConiferChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<LiamsConiferChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<LiamsConiferChain> for LiamsConiferSbs {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = LiamsConiferSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();

		assert_eq!(proto.stalk, sbs.scale.to_stalk());
		assert_eq!(proto.first_ring_unit_height, sbs.rings.height_range.start);
		assert_eq!(proto.ring_spacing_unit_height, sbs.rings.spacing);
		assert_eq!(proto.anchors_per_ring, sbs.rings.anchors_per_ring);
		assert_eq!(
			proto.max_projection_fraction_of_height,
			sbs.projection.length_fraction_of_height.start
		);
		assert_eq!(proto.branch_depth, sbs.growth.branch_depth);
	}

	#[test]
	fn tuft_world_scale_scales_with_stalk_height() {
		let low = LiamsConiferSbs {
			scale: LiamsConiferScale { stalk_height: 20.0, ..Default::default() },
			tuft_scale_factor: 0.02,
			..Default::default()
		};
		let high = LiamsConiferSbs {
			scale: LiamsConiferScale { stalk_height: 40.0, ..Default::default() },
			tuft_scale_factor: 0.02,
			..Default::default()
		};
		assert!(high.tuft_world_scale() > low.tuft_world_scale());
	}

	#[test]
	fn build_chain_has_stalk_and_branches() -> anyhow::Result<()> {
		let chain = LiamsConiferSbs::default().build_chain();
		assert!(chain.nodes.len() > 10);
		Ok(())
	}

	#[test]
	fn mini_sapling_perturbation_scales_with_stalk() -> Result<()> {
		let mut mini = LiamsConiferSbs::default();
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
