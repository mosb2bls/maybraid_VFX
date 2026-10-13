//! Restricted **Honu Banyan** geometry for CLI and playgrounds ([#250](https://github.com/ramate-io/maybraid/issues/250)).

#[cfg(feature = "clap")]
use procedural_common::noise_params_from_scalar_str;
#[cfg(any(feature = "clap", test))]
use procedural_common::{
	parse_count_pair as parse_ring_layout, parse_unit_range, parse_usize_range as parse_depth_range,
};
use procedural_common::{
	CountPair as RingLayout, NoiseConfig, NoiseParams, UnitRange, UsizeRange as DepthRange,
};

use crate::anchors::honu_banyan::{
	HonuBanyanAnchorPerturbation, HonuBanyanAnchors, HonuBanyanProtoAnchors,
	DEFAULT_DESCENDER_THRESHOLD, DEFAULT_FIRST_RING_HEIGHT_FRACTION,
	DEFAULT_LAST_RING_HEIGHT_FRACTION, DEFAULT_MAX_DEPTH_FIRST_RING, DEFAULT_MAX_DEPTH_LAST_RING,
	DEFAULT_PROJECTION_MAX_FRACTION, DEFAULT_PROJECTION_MIN_FRACTION, DEFAULT_PROJECTION_MIX_SCALE,
	DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_STALK_RADIUS_FRACTION, DEFAULT_STALK_SECTION_COUNT,
	DEFAULT_TREE_HEIGHT,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::{BallStickChain, HonuBanyanChain};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HonuBanyanScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TREE_HEIGHT))]
	pub tree_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT_FRACTION))]
	pub stalk_height_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_RADIUS_FRACTION))]
	pub stalk_radius_fraction: f32,
}

impl Default for HonuBanyanScale {
	fn default() -> Self {
		Self {
			tree_height: DEFAULT_TREE_HEIGHT,
			stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_radius_fraction: DEFAULT_STALK_RADIUS_FRACTION,
		}
	}
}

impl HonuBanyanScale {
	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk {
			stalk_height: self.tree_height * self.stalk_height_fraction,
			stalk_base_radius: self.tree_height * self.stalk_radius_fraction,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct RingAnchorParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "rings",
			default_value = "3x4",
			value_parser = parse_ring_layout,
			value_name = "RINGSXANCHORS"
		)
	)]
	pub layout: RingLayout,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.80..0.95",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
}

impl Default for RingAnchorParams {
	fn default() -> Self {
		Self {
			layout: RingLayout::new(3, 4),
			height_range: UnitRange::new(
				DEFAULT_FIRST_RING_HEIGHT_FRACTION,
				DEFAULT_LAST_RING_HEIGHT_FRACTION,
			),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HonuProjectionParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.55..1.05",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub length_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_PROJECTION_MIX_SCALE))]
	pub mix_scale: f32,
}

impl Default for HonuProjectionParams {
	fn default() -> Self {
		Self {
			length_fraction_of_height: UnitRange::new(
				DEFAULT_PROJECTION_MIN_FRACTION,
				DEFAULT_PROJECTION_MAX_FRACTION,
			),
			mix_scale: DEFAULT_PROJECTION_MIX_SCALE,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HonuGrowthParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "depth",
			default_value = "3..5",
			value_parser = parse_depth_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub depth: DepthRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_DESCENDER_THRESHOLD))]
	pub descender_threshold: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_SECTION_COUNT))]
	pub stalk_section_count: u32,
}

impl Default for HonuGrowthParams {
	fn default() -> Self {
		Self {
			depth: DepthRange::new(DEFAULT_MAX_DEPTH_FIRST_RING, DEFAULT_MAX_DEPTH_LAST_RING),
			descender_threshold: DEFAULT_DESCENDER_THRESHOLD,
			stalk_section_count: DEFAULT_STALK_SECTION_COUNT,
		}
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
			default_value = "-0.5..0.5",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub vertical_offset: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-angular-perturbation",
			default_value = "0.0..0.25",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub angular_scale: UnitRange,
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "anchor-radius-perturbation",
			default_value = "-0.04..0.04",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub radius_offset: UnitRange,
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
			vertical_offset: UnitRange::new(-0.5, 0.5),
			angular_scale: UnitRange::new(0.0, 0.25),
			radius_offset: UnitRange::new(-0.04, 0.04),
			noise: NoiseParams::default(),
		}
	}
}

impl AnchorPerturbationParams {
	pub fn to_perturbation(&self) -> HonuBanyanAnchorPerturbation {
		HonuBanyanAnchorPerturbation {
			noise: self.noise,
			vertical_offset: self.vertical_offset.start..self.vertical_offset.end,
			angular_scale: self.angular_scale.start..self.angular_scale.end,
			radius_offset: self.radius_offset.start..self.radius_offset.end,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HonuBanyanSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: HonuBanyanScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchors"))]
	pub rings: RingAnchorParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: HonuProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: HonuGrowthParams,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = 0.10, help_heading = "Terminal canopy")
	)]
	pub leaf_ball_factor: f32,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: AnchorPerturbationParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
}

impl Default for HonuBanyanSbs {
	fn default() -> Self {
		Self {
			scale: HonuBanyanScale::default(),
			rings: RingAnchorParams::default(),
			projection: HonuProjectionParams::default(),
			growth: HonuGrowthParams::default(),
			leaf_ball_factor: 0.10,
			anchor_perturbation: AnchorPerturbationParams::default(),
			canopy_noise: NoiseParams::default(),
		}
	}
}

impl HonuBanyanSbs {
	pub fn crown_floor_world_y(&self) -> f32 {
		self.scale.tree_height * self.rings.height_range.start
	}

	pub fn leaf_ball_size(&self) -> f32 {
		self.scale.tree_height * self.leaf_ball_factor
	}

	pub fn to_proto(&self) -> HonuBanyanProtoAnchors {
		HonuBanyanProtoAnchors {
			tree_height: self.scale.tree_height,
			stalk: self.scale.to_stalk(),
			first_ring_height_fraction: self.rings.height_range.start,
			last_ring_height_fraction: self.rings.height_range.end,
			ring_count: self.rings.layout.first,
			anchors_per_ring: self.rings.layout.second,
			projection_min_fraction: self.projection.length_fraction_of_height.start,
			projection_max_fraction: self.projection.length_fraction_of_height.end,
			projection_mix_scale: self.projection.mix_scale,
			max_depth_first_ring: self.growth.depth.start,
			max_depth_last_ring: self.growth.depth.end,
			descender_threshold: self.growth.descender_threshold,
			stalk_section_count: self.growth.stalk_section_count,
		}
	}

	pub fn to_anchors(&self) -> HonuBanyanAnchors {
		HonuBanyanAnchors::new(self.to_proto())
			.with_perturbation(self.anchor_perturbation.to_perturbation())
	}

	pub fn hysteresis_seeds(&self) -> Vec<HonuBanyanChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<HonuBanyanChain> {
		AnchorsToChain::build_chain(self)
	}

	/// Mini Honu for understory / thicket groves (~2–4 m).
	pub fn apply_mini_honu_preset(&mut self) {
		self.scale.tree_height = 3.0;
		self.rings.layout = RingLayout::new(2, 5);
		self.rings.height_range = UnitRange::new(0.78, 0.92);
		self.growth.depth = DepthRange::new(4, 5);
		self.leaf_ball_factor = 0.12;
	}
}

impl Anchors<HonuBanyanChain> for HonuBanyanSbs {
	fn anchors(&self) -> Vec<HonuBanyanChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_frontend_converts_to_anchor_recipe() {
		let sbs = HonuBanyanSbs::default();
		let anchors = sbs.to_anchors();
		let proto = anchors.proto();
		assert_eq!(proto.tree_height, sbs.scale.tree_height);
		assert_eq!(proto.ring_count, 3);
		assert_eq!(proto.first_ring_height_fraction, 0.80);
	}

	#[test]
	fn build_chain_has_many_nodes() -> anyhow::Result<()> {
		let chain = HonuBanyanSbs::default().build_chain();
		assert!(chain.nodes.len() > 20);
		Ok(())
	}

	#[test]
	fn mini_preset_shrinks_height() {
		let mut sbs = HonuBanyanSbs::default();
		sbs.apply_mini_honu_preset();
		assert_eq!(sbs.scale.tree_height, 3.0);
	}

	#[test]
	fn default_chain_emits_descenders() {
		use crate::chain::honu_banyan::HonuBanyanPhase;

		let chain = HonuBanyanSbs::default().build_chain();
		let descenders = chain
			.nodes_with_hysteresis_enumerated()
			.filter(|(_, _, h)| {
				matches!(
					h.phase,
					HonuBanyanPhase::StartDescender(_) | HonuBanyanPhase::EndDescender(_)
				)
			})
			.count();
		assert!(
			descenders > 0,
			"expected descender phases in default Honu chain (nodes={})",
			chain.nodes.len()
		);
	}
}
