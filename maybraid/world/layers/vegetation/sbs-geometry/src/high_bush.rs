//! High-bush shoot shape IR ([RFC §3.1.6.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/03-high-bushes-and-shoots/README.md)).

use crate::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_BIAS_BLEND, DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
	DEFAULT_HEIGHT, DEFAULT_RADIAL_STRENGTH, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO, DEFAULT_SHOOT_COUNT, DEFAULT_VERTICAL_BIAS,
};
use crate::{
	high_bush_branch_depth, high_bush_is_graph_terminal, BallStickChain, HighBushChain,
	HighBushPhase, HighBushProtoAnchors,
};
use procedural_common::{NoiseConfig, NoiseParams};

/// Terminal foliage style for composing recipes.
///
/// VegetationComponents maps every variant onto ball kits (cheap or layered).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
pub enum HighBushFoliageStyle {
	PlaneSplay,
	Tuft,
	CheapBall,
	#[default]
	LayeredBall,
}

/// Configurable high-bush construction.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct HighBushShootsShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10.0))]
	pub height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.02))]
	pub anchor_lift_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub shoot_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.35))]
	pub radial_strength: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub vertical_bias: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub segment_length_fraction_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.16))]
	pub segment_length_fraction_hi: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.012))]
	pub segment_radius_fraction_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.025))]
	pub segment_radius_fraction_hi: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.05))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, value_enum, default_value_t = HighBushFoliageStyle::LayeredBall))]
	pub foliage_style: HighBushFoliageStyle,
	#[cfg_attr(
		feature = "clap",
		arg(
			long,
			default_value = "0,1,1,1",
			value_parser = procedural_common::noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
		)
	)]
	pub chain_noise: NoiseParams,
}

impl HighBushShootsShape {
	pub fn to_proto(&self) -> HighBushProtoAnchors {
		HighBushProtoAnchors {
			height: self.height,
			anchor_lift_fraction: self.anchor_lift_fraction,
			shoot_count: self.shoot_count,
			radial_strength: self.radial_strength,
			vertical_bias: self.vertical_bias,
			branch_depth: high_bush_branch_depth(self.branch_depth),
			child_count_min: 1,
			child_count_max: 2,
			angle_tolerance_radians: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: DEFAULT_BIAS_BLEND,
			segment_length_fraction_lo: self.segment_length_fraction_lo,
			segment_length_fraction_hi: self.segment_length_fraction_hi,
			segment_radius_fraction_lo: self.segment_radius_fraction_lo,
			segment_radius_fraction_hi: self.segment_radius_fraction_hi,
			root_radius_fraction_of_height: 0.018,
			branch_radius_child_scale: (0.72, 0.80),
		}
	}

	pub fn build_chain(&self) -> BallStickChain<HighBushChain> {
		let noise = NoiseConfig::new(self.chain_noise);
		BallStickChain::build(self.to_proto().hysteresis_seeds(noise))
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.height * self.leaf_radius_fraction
	}
}

impl Default for HighBushShootsShape {
	fn default() -> Self {
		common_high_bush_shape()
	}
}

/// RFC §3.1.7.12 ball selection for Common High Bush.
pub fn should_allocate_foliage(
	node_idx: usize,
	hysteresis: &HighBushChain,
	chain: &BallStickChain<HighBushChain>,
) -> bool {
	if matches!(hysteresis.phase, HighBushPhase::Root { .. }) {
		return false;
	}
	high_bush_is_graph_terminal(chain, node_idx)
		|| hysteresis.height_fraction() > 0.45
		|| hysteresis.branch_order() > 1
}

/// RFC shoot count band for Common High Bush.
pub const COMMON_HIGH_BUSH_SHOOT_COUNT: std::ops::RangeInclusive<u32> = 7..=10;
pub const COMMON_HIGH_BUSH_RADIAL_STRENGTH: f32 = DEFAULT_RADIAL_STRENGTH;
pub const COMMON_HIGH_BUSH_VERTICAL_BIAS: f32 = DEFAULT_VERTICAL_BIAS;
/// Leafy bush world radius as a fraction of `H`.
pub const COMMON_HIGH_BUSH_LEAF_RADIUS_FRACTION: f32 = 0.05;

fn common_high_bush_shape() -> HighBushShootsShape {
	HighBushShootsShape {
		height: DEFAULT_HEIGHT,
		shoot_count: DEFAULT_SHOOT_COUNT,
		radial_strength: COMMON_HIGH_BUSH_RADIAL_STRENGTH,
		vertical_bias: COMMON_HIGH_BUSH_VERTICAL_BIAS,
		anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
		branch_depth: 4,
		segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
		segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
		segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
		segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
		leaf_radius_fraction: COMMON_HIGH_BUSH_LEAF_RADIUS_FRACTION,
		foliage_style: HighBushFoliageStyle::LayeredBall,
		chain_noise: NoiseParams::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_shape_builds_chain() -> anyhow::Result<()> {
		let chain = HighBushShootsShape::default().build_chain();
		assert!(chain.nodes.len() > 20);
		Ok(())
	}

	#[test]
	fn preset_shoot_count_in_rfc_band() {
		let shape = HighBushShootsShape::default();
		assert!(COMMON_HIGH_BUSH_SHOOT_COUNT.contains(&shape.shoot_count));
	}
}
