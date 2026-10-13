//! Restricted **Vase Tree** geometry for CLI and playgrounds ([#246](https://github.com/ramate-io/maybraid/issues/246)).

#[cfg(feature = "clap")]
use procedural_common::{noise_params_from_scalar_str, parse_unit_range};
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::vase_tree::{
	VaseTreeAnchorPerturbation, VaseTreeAnchors, VaseTreeProtoAnchors, BUSH_STALK_HEIGHT_FRACTION,
	DEFAULT_ANCHORS_PER_RING, DEFAULT_BIAS_ELEVATION_HI_DEGREES, DEFAULT_BIAS_ELEVATION_LO_DEGREES,
	DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES, DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
	DEFAULT_BRANCH_DEPTH, DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI,
	DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO, DEFAULT_CHILD_COUNT_MAX, DEFAULT_CHILD_COUNT_MIN,
	DEFAULT_FIRST_RING_UNIT_HEIGHT, DEFAULT_LAST_RING_UNIT_HEIGHT, DEFAULT_LEAF_RADIUS_FRACTION,
	DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION, DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
	DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT, DEFAULT_RING_SPACING_UNIT_HEIGHT,
	DEFAULT_STALK_BASE_RADIUS_FRACTION, DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_TREE_HEIGHT,
	DEFAULT_UPPER_FOLIAGE_RING_U, DEFAULT_VASE_PROFILE_CENTER, DEFAULT_VASE_PROFILE_EPSILON,
};
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::scale::{
	leaf_radius_for_stalk_scale, outer_foliage_distance_for_stalk, stalk_radius_scaled_range,
	stalk_scaled_range,
};
use crate::BallStickChain;
use crate::StorybookTreeChain;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TREE_HEIGHT))]
	pub tree_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT_FRACTION))]
	pub stalk_height_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for VaseTreeScale {
	fn default() -> Self {
		Self {
			tree_height: DEFAULT_TREE_HEIGHT,
			stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_base_radius: None,
		}
	}
}

impl VaseTreeScale {
	pub fn stalk_height(&self) -> f32 {
		self.tree_height.max(1e-6) * self.stalk_height_fraction
	}

	pub fn stalk_base_radius_or_default(&self) -> f32 {
		self.stalk_base_radius
			.unwrap_or(DEFAULT_STALK_BASE_RADIUS_FRACTION * self.tree_height)
	}

	pub fn to_stalk(&self) -> StrictStalk {
		StrictStalk {
			stalk_height: self.stalk_height(),
			stalk_base_radius: self.stalk_base_radius_or_default(),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeRingParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.2667..1.0",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_RING_SPACING_UNIT_HEIGHT))]
	pub spacing: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_ANCHORS_PER_RING))]
	pub anchors_per_ring: u32,
}

impl Default for VaseTreeRingParams {
	fn default() -> Self {
		Self {
			height_range: UnitRange::new(
				DEFAULT_FIRST_RING_UNIT_HEIGHT,
				DEFAULT_LAST_RING_UNIT_HEIGHT,
			),
			spacing: DEFAULT_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: DEFAULT_ANCHORS_PER_RING,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeProjectionParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.15..0.50",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub span_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_VASE_PROFILE_EPSILON))]
	pub vase_profile_epsilon: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_VASE_PROFILE_CENTER))]
	pub vase_profile_center: f32,
}

impl VaseTreeProjectionParams {
	pub fn min_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.min(self.span_fraction_of_height.end)
	}

	pub fn max_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.max(self.span_fraction_of_height.end)
	}
}

impl Default for VaseTreeProjectionParams {
	fn default() -> Self {
		Self {
			span_fraction_of_height: UnitRange::new(
				DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
				DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			),
			vase_profile_epsilon: DEFAULT_VASE_PROFILE_EPSILON,
			vase_profile_center: DEFAULT_VASE_PROFILE_CENTER,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeGrowthParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_BRANCH_DEPTH))]
	pub branch_depth: usize,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES)
	)]
	pub angle_tolerance_degrees: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_BIAS_ELEVATION_LO_DEGREES)
	)]
	pub bias_elevation_lo_degrees: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_BIAS_ELEVATION_HI_DEGREES)
	)]
	pub bias_elevation_hi_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_CHILD_COUNT_MIN))]
	pub child_count_min: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_CHILD_COUNT_MAX))]
	pub child_count_max: u32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK)
	)]
	pub branch_base_radius_fraction_of_stalk: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO))]
	pub branch_radius_child_scale_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI))]
	pub branch_radius_child_scale_hi: f32,
}

impl Default for VaseTreeGrowthParams {
	fn default() -> Self {
		Self {
			branch_depth: DEFAULT_BRANCH_DEPTH,
			angle_tolerance_degrees: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
			bias_elevation_lo_degrees: DEFAULT_BIAS_ELEVATION_LO_DEGREES,
			bias_elevation_hi_degrees: DEFAULT_BIAS_ELEVATION_HI_DEGREES,
			child_count_min: DEFAULT_CHILD_COUNT_MIN,
			child_count_max: DEFAULT_CHILD_COUNT_MAX,
			branch_base_radius_fraction_of_stalk: DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale_lo: DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO,
			branch_radius_child_scale_hi: DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeCanopyParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_LEAF_RADIUS_FRACTION))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_UPPER_FOLIAGE_RING_U))]
	pub upper_foliage_ring_u: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION)
	)]
	pub outer_foliage_distance_fraction: f32,
}

impl Default for VaseTreeCanopyParams {
	fn default() -> Self {
		Self {
			leaf_radius_fraction: DEFAULT_LEAF_RADIUS_FRACTION,
			upper_foliage_ring_u: DEFAULT_UPPER_FOLIAGE_RING_U,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct VaseTreeAnchorPerturbationParams {
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
			default_value = "1337,1,1,1",
			value_parser = noise_params_from_scalar_str,
			value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
		)
	)]
	pub noise: NoiseParams,
}

impl Default for VaseTreeAnchorPerturbationParams {
	fn default() -> Self {
		let p = VaseTreeAnchorPerturbation::default();
		Self {
			vertical_offset: UnitRange::new(p.vertical_offset.start, p.vertical_offset.end),
			angular_scale: UnitRange::new(p.angular_scale.start, p.angular_scale.end),
			radius_offset: UnitRange::new(p.radius_offset.start, p.radius_offset.end),
			noise: p.noise,
		}
	}
}

impl VaseTreeAnchorPerturbationParams {
	pub fn to_perturbation(&self) -> VaseTreeAnchorPerturbation {
		VaseTreeAnchorPerturbation {
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
pub struct VaseTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: VaseTreeScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Rings"))]
	pub rings: VaseTreeRingParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: VaseTreeProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: VaseTreeGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy"))]
	pub canopy: VaseTreeCanopyParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: VaseTreeAnchorPerturbationParams,
}

impl Default for VaseTreeSbs {
	fn default() -> Self {
		Self {
			scale: VaseTreeScale::default(),
			rings: VaseTreeRingParams::default(),
			projection: VaseTreeProjectionParams::default(),
			growth: VaseTreeGrowthParams::default(),
			canopy: VaseTreeCanopyParams::default(),
			canopy_noise: NoiseParams::default(),
			anchor_perturbation: VaseTreeAnchorPerturbationParams::default(),
		}
	}
}

impl VaseTreeSbs {
	pub fn height(&self) -> f32 {
		self.scale.tree_height.max(1e-6)
	}

	pub fn leaf_radius_world(&self) -> f32 {
		leaf_radius_for_stalk_scale(
			self.height(),
			self.canopy.leaf_radius_fraction,
			self.scale.stalk_height(),
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
			DEFAULT_TREE_HEIGHT,
		)
	}

	pub fn apex_radius_world(&self, apex_ball_radius_fraction_of_height: f32) -> f32 {
		leaf_radius_for_stalk_scale(
			self.height(),
			apex_ball_radius_fraction_of_height,
			self.scale.stalk_height(),
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
			DEFAULT_TREE_HEIGHT,
		)
	}

	/// Bush variant: shorter stalk (RFC ornamental / grape-vine form).
	pub fn apply_bush_preset(&mut self) {
		self.scale.stalk_height_fraction = BUSH_STALK_HEIGHT_FRACTION;
	}

	pub fn to_proto(&self) -> VaseTreeProtoAnchors {
		VaseTreeProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			projection_min_fraction_of_height: self.projection.min_fraction(),
			projection_max_fraction_of_height: self.projection.max_fraction(),
			vase_profile_epsilon: self.projection.vase_profile_epsilon,
			vase_profile_center: self.projection.vase_profile_center,
			bias_elevation_lo_degrees: self.growth.bias_elevation_lo_degrees,
			bias_elevation_hi_degrees: self.growth.bias_elevation_hi_degrees,
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			branch_depth: self.growth.branch_depth,
			child_count_min: self.growth.child_count_min,
			child_count_max: self.growth.child_count_max.max(self.growth.child_count_min),
			upper_foliage_ring_u: self.canopy.upper_foliage_ring_u,
			outer_foliage_distance_fraction: outer_foliage_distance_for_stalk(
				self.canopy.outer_foliage_distance_fraction,
				self.scale.stalk_height(),
				DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
			),
			branch_base_radius_fraction_of_stalk: self.growth.branch_base_radius_fraction_of_stalk,
			branch_radius_child_scale: (
				self.growth.branch_radius_child_scale_lo,
				self.growth.branch_radius_child_scale_hi,
			),
			..VaseTreeProtoAnchors::default()
		}
	}

	pub fn to_anchors(&self) -> VaseTreeAnchors {
		let mut perturbation = self.anchor_perturbation.to_perturbation();
		let scaled = stalk_scaled_range(
			UnitRange::new(perturbation.vertical_offset.start, perturbation.vertical_offset.end),
			self.scale.stalk_height(),
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
		);
		perturbation.vertical_offset = scaled.start..scaled.end;
		let radius_scaled = stalk_radius_scaled_range(
			UnitRange::new(perturbation.radius_offset.start, perturbation.radius_offset.end),
			self.scale.stalk_base_radius_or_default(),
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_BASE_RADIUS_FRACTION,
		);
		perturbation.radius_offset = radius_scaled.start..radius_scaled.end;
		VaseTreeAnchors::new(self.to_proto()).with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<StorybookTreeChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for VaseTreeSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::vase_tree::DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT;
	use anyhow::Result;

	#[test]
	fn default_frontend_converts_to_proto() -> Result<()> {
		let sbs = VaseTreeSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.tree_height - DEFAULT_TREE_HEIGHT).abs() < 1e-5);
		assert!(
			(proto.stalk.stalk_height - DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION).abs()
				< 1e-4
		);
		assert_eq!(proto.branch_depth, DEFAULT_BRANCH_DEPTH);
		assert_eq!(proto.anchors_per_ring, DEFAULT_ANCHORS_PER_RING);
		Ok(())
	}

	#[test]
	fn build_chain_produces_substantial_graph() -> Result<()> {
		let chain = VaseTreeSbs::default().build_chain();
		assert!(chain.nodes.len() > 40, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn bush_preset_shortens_stalk() -> Result<()> {
		let mut sbs = VaseTreeSbs::default();
		sbs.apply_bush_preset();
		let proto = sbs.to_proto();
		assert!(
			(proto.stalk.stalk_height - DEFAULT_TREE_HEIGHT * BUSH_STALK_HEIGHT_FRACTION).abs()
				< 1e-4
		);
		Ok(())
	}

	#[test]
	fn apex_radius_scales_for_mini_forms() -> Result<()> {
		let full = VaseTreeSbs::default();
		let mut mini = VaseTreeSbs::default();
		mini.scale.tree_height = 1.8;
		let fraction = DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT;
		assert!((full.apex_radius_world(fraction) - full.height() * fraction).abs() < 1e-4);
		assert!(mini.apex_radius_world(fraction) / mini.height() > fraction);
		Ok(())
	}
}
