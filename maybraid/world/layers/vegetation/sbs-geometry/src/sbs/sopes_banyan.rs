//! Restricted **Sope's Banyan** geometry for CLI and playgrounds.

#[cfg(feature = "clap")]
use procedural_common::noise_params_from_scalar_str;
#[cfg(any(feature = "clap", test))]
use procedural_common::{
	parse_count_pair as parse_ring_layout, parse_unit_range, parse_usize_range as parse_depth_range,
};
use procedural_common::{
	CountPair as RingLayout, NoiseConfig, NoiseParams, UnitRange, UsizeRange as DepthRange,
};

use crate::anchors::sopes_banyan::{
	SopesBanyanAnchorPerturbation, SopesBanyanAnchors, SopesBanyanProtoAnchors,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::{BallStickChain, SopesBanyanChain};

/// High-level world scale for the art-directed Sope's Banyan recipe.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanScale {
	/// Height of the strict vertical stalk in world units.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 20.0))]
	pub stalk_height: f32,
	/// Height used by canopy phases for descender length and overall banyan scale.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 40.0))]
	pub canopy_height: f32,
	/// Radius of the stalk base and radial scale for anchor offsets.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.75))]
	pub stalk_base_radius: f32,
}

impl Default for SopesBanyanScale {
	fn default() -> Self {
		Self { stalk_height: 20.0, canopy_height: 40.0, stalk_base_radius: 0.75 }
	}
}

impl SopesBanyanScale {
	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk { stalk_height: self.stalk_height, stalk_base_radius: self.stalk_base_radius }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct RingAnchorParams {
	/// Ring count and anchors per ring as `ringsxanchors`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "rings",
			default_value = "8x7",
			value_parser = parse_ring_layout,
			value_name = "RINGSXANCHORS"
		)
	)]
	pub layout: RingLayout,
	/// First and last ring heights as fractions of the stalk height: `first..last`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.40..0.95",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
}

impl Default for RingAnchorParams {
	fn default() -> Self {
		Self { layout: RingLayout::new(8, 7), height_range: UnitRange::new(0.40, 0.95) }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseProjectionParams {
	/// Min and max projection length as fractions of stalk height: `min..max`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.05..0.18",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub length_fraction_of_height: UnitRange,
	/// Clamp epsilon for the bounded logit vase profile.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.4))]
	pub profile_epsilon: f32,
	/// Center point of the vase projection profile in normalized ring height.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.5))]
	pub center_fraction: f32,
}

impl Default for VaseProjectionParams {
	fn default() -> Self {
		Self {
			length_fraction_of_height: UnitRange::new(0.05, 0.18),
			profile_epsilon: 0.4,
			center_fraction: 0.5,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct CanopyGrowthParams {
	/// First and last ring depth budgets as `first..last`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "depth",
			default_value = "1..8",
			value_parser = parse_depth_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub depth: DepthRange,
	/// Noise threshold below which branch candidates become descenders.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.15))]
	pub descender_threshold: f32,
}

impl Default for CanopyGrowthParams {
	fn default() -> Self {
		Self { depth: DepthRange::new(1, 8), descender_threshold: 0.15 }
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct AnchorPerturbationParams {
	/// Vertical anchor offset in world units as `min..max`.
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
	/// Direction perturbation scale as `min..max` passed through the shared degree-range perturbation.
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
	/// Anchor radius offset in world units as `min..max`.
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
	/// Noise used only for anchor perturbation sampling as `seed,frequency,amplitude,octaves`.
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-perturbation-noise",
			default_value = "1337,1,1,1",
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
			radius_offset: UnitRange::new(-0.05, 0.05),
			noise: NoiseParams::default(),
		}
	}
}

impl AnchorPerturbationParams {
	pub fn to_perturbation(&self) -> SopesBanyanAnchorPerturbation {
		SopesBanyanAnchorPerturbation {
			noise: self.noise,
			vertical_offset: self.vertical_offset.start..self.vertical_offset.end,
			angular_scale: self.angular_scale.start..self.angular_scale.end,
			radius_offset: self.radius_offset.start..self.radius_offset.end,
		}
	}
}

/// Art-directed front-end: scale + rings + projection + growth + one structural canopy noise.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyanSbs {
	/// High-level world-space scale controls.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: SopesBanyanScale,
	/// Ring count, spoke count, and vertical ring band.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchors"))]
	pub rings: RingAnchorParams,
	/// Vase projection profile for initial canopy spokes.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: VaseProjectionParams,
	/// Chain depth and descender controls.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: CanopyGrowthParams,
	/// Dimensionless terminal leaf scale; world radius from [`Self::leaf_ball_size`].
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = 0.4, help_heading = "Terminal canopy")
	)]
	pub leaf_ball_factor: f32,
	/// Perturbation applied to non-stalk anchors after deterministic ring generation.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: AnchorPerturbationParams,
	/// Structural canopy noise used by branch and descender decisions.
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
}

impl Default for SopesBanyanSbs {
	fn default() -> Self {
		Self {
			scale: SopesBanyanScale::default(),
			rings: RingAnchorParams::default(),
			projection: VaseProjectionParams::default(),
			growth: CanopyGrowthParams::default(),
			leaf_ball_factor: 0.4,
			anchor_perturbation: AnchorPerturbationParams::default(),
			canopy_noise: NoiseParams::default(),
		}
	}
}

impl SopesBanyanSbs {
	/// World `y` of the crown floor (first ring height), matching terminal foliage cut-in for render.
	pub fn crown_floor_world_y(&self) -> f32 {
		self.scale.stalk_height * self.rings.height_range.start
	}

	/// Suggested world-space radius for terminal leaf balls / splays (uniform scale numerator = this / [`crate::BallStickNode::radius`]).
	///
	/// Uses `leaf_ball_factor × canopy_height` and a fixed in-crown height weight (smoothstep) so the same factor stays in a sensible range when [`SopesBanyanScale::canopy_height`] changes.
	pub fn leaf_ball_size(&self) -> f32 {
		self.scale.stalk_height * self.leaf_ball_factor
	}

	/// Full anchor recipe (stalk + rings); chain noise is applied when emitting seeds.
	pub fn to_anchors(&self) -> SopesBanyanAnchors {
		SopesBanyanAnchors::new(SopesBanyanProtoAnchors {
			stalk: self.scale.to_stalk(),
			descender_threshold: self.growth.descender_threshold,
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_count: self.rings.layout.first,
			anchors_per_ring: self.rings.layout.second,
			projection_min_fraction_of_height: self.projection.length_fraction_of_height.start,
			projection_max_fraction_of_height: self.projection.length_fraction_of_height.end,
			vase_profile_epsilon: self.projection.profile_epsilon,
			projection_center_fraction: self.projection.center_fraction,
			max_depth_first_ring: self.growth.depth.start,
			max_depth_last_ring: self.growth.depth.end,
		})
		.with_perturbation(self.anchor_perturbation.to_perturbation())
	}

	/// Canopy [`SopesBanyanChain`] seeds using this struct's shared [`NoiseParams`].
	pub fn hysteresis_seeds(&self) -> Vec<SopesBanyanChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(
			noise,
			self.scale.canopy_height,
			self.growth.descender_threshold,
		)
	}

	pub fn build_chain(&self) -> BallStickChain<SopesBanyanChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<SopesBanyanChain> for SopesBanyanSbs {
	fn anchors(&self) -> Vec<SopesBanyanChain> {
		self.hysteresis_seeds()
	}
}

impl SopesBanyanSbs {
	/// Override the canopy noise (callers vary the seed per placement).
	pub fn with_noise_params(mut self, params: NoiseParams) -> Self {
		self.canopy_noise = params;
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compact_parsers_accept_expected_shapes() -> anyhow::Result<()> {
		assert_eq!(
			parse_ring_layout("8x7").map_err(|e| anyhow::anyhow!("{e}"))?,
			RingLayout::new(8, 7)
		);
		assert_eq!(
			parse_unit_range("0.40..0.95").map_err(|e| anyhow::anyhow!("{e}"))?,
			UnitRange::new(0.40, 0.95)
		);
		assert_eq!(
			parse_depth_range("4..6").map_err(|e| anyhow::anyhow!("{e}"))?,
			DepthRange::new(4, 6)
		);
		Ok(())
	}

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = SopesBanyanSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();

		assert_eq!(proto.stalk, sbs.scale.to_stalk());
		assert_eq!(proto.ring_count, sbs.rings.layout.first);
		assert_eq!(proto.anchors_per_ring, sbs.rings.layout.second);
		assert_eq!(proto.first_ring_unit_height, sbs.rings.height_range.start);
		assert_eq!(proto.last_ring_unit_height, sbs.rings.height_range.end);
		assert_eq!(
			proto.projection_min_fraction_of_height,
			sbs.projection.length_fraction_of_height.start
		);
		assert_eq!(
			proto.projection_max_fraction_of_height,
			sbs.projection.length_fraction_of_height.end
		);
		assert_eq!(proto.max_depth_first_ring, sbs.growth.depth.start);
		assert_eq!(proto.max_depth_last_ring, sbs.growth.depth.end);
		assert_eq!(
			anchors.perturbation.vertical_offset,
			sbs.anchor_perturbation.vertical_offset.start
				..sbs.anchor_perturbation.vertical_offset.end
		);
	}

	#[test]
	fn scale_controls_stalk_and_canopy_height() {
		let sbs = SopesBanyanSbs {
			scale: SopesBanyanScale {
				stalk_height: 12.0,
				canopy_height: 30.0,
				..Default::default()
			},
			..Default::default()
		};
		let seeds = sbs.hysteresis_seeds();

		assert_eq!(sbs.to_anchors().proto().stalk.stalk_height, 12.0);
		assert!(seeds.iter().all(|seed| seed.stalk_height == 12.0));
		assert!(seeds.iter().all(|seed| seed.banyan_height == 30.0));
	}

	#[test]
	fn unit_stalk_chain_scales_from_default() {
		let full = SopesBanyanSbs::default().build_chain();
		let mut unit = SopesBanyanSbs::default();
		unit.scale.stalk_height = 1.0;
		unit.scale.canopy_height = 2.0;
		unit.scale.stalk_base_radius = 0.75 / 20.0;
		let unit = unit.build_chain();
		let full_max_y = full.nodes.iter().map(|n| n.position.y).fold(0.0f32, f32::max);
		let unit_max_y = unit.nodes.iter().map(|n| n.position.y).fold(0.0f32, f32::max);
		assert!(unit_max_y < 2.5, "unit Sope crown should stay near unit height, got {unit_max_y}");
		assert!(
			(full_max_y / 20.0 - unit_max_y).abs() < 0.75,
			"default max y {full_max_y} should be ~20× unit {unit_max_y}"
		);
	}

	#[test]
	fn noise_params_override_frontend_noise() {
		let params = NoiseParams { seed: 99, ..Default::default() };
		let sbs = SopesBanyanSbs::default().with_noise_params(params);
		assert_eq!(sbs.canopy_noise.seed, 99);
	}

	#[test]
	fn leaf_ball_size_scales_with_canopy_height() {
		let low = SopesBanyanSbs {
			scale: SopesBanyanScale { stalk_height: 20.0, ..Default::default() },
			leaf_ball_factor: 0.2,
			..Default::default()
		};
		let high = SopesBanyanSbs {
			scale: SopesBanyanScale { stalk_height: 40.0, ..Default::default() },
			leaf_ball_factor: 0.2,
			..Default::default()
		};
		assert!(low.leaf_ball_size() > 0.0);
		assert!(high.leaf_ball_size() > low.leaf_ball_size());
	}
}
