//! Restricted **Penmarch Torch** geometry for CLI and playgrounds ([#248](https://github.com/ramate-io/maybraid/issues/248)).

#[cfg(feature = "clap")]
use procedural_common::{noise_params_from_scalar_str, parse_unit_range};
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use crate::anchors::penmarch_torch::{
	PenmarchTorchAnchorPerturbation, PenmarchTorchAnchors, PenmarchTorchProtoAnchors,
	DEFAULT_APEX_ONLY_FLIP_FRACTION, DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
	DEFAULT_CROWN_ELEVATION_DEGREES, DEFAULT_CROWN_FLIP_RING_U, DEFAULT_FIRST_RING_UNIT_HEIGHT,
	DEFAULT_FLARE_ELEVATION_DEGREES, DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
	DEFAULT_PROJECTION_CENTER_FRACTION, DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
	DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT, DEFAULT_SHOULDER_ELEVATION_DEGREES,
	DEFAULT_STALK_BASE_RADIUS_FRACTION, DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_TREE_HEIGHT,
	DEFAULT_VASE_PROFILE_EPSILON,
};
use crate::anchors::strict_stalk::StrictStalk;
use crate::anchors::torch_tree::{
	torch_ring_spacing_unit_height, TORCH_ANCHORS_PER_RING, TORCH_ANCHOR_ANGULAR_SCALE_HI,
	TORCH_ANCHOR_ANGULAR_SCALE_LO, TORCH_ANCHOR_RADIUS_OFFSET_HI, TORCH_ANCHOR_RADIUS_OFFSET_LO,
	TORCH_ANCHOR_VERTICAL_OFFSET_HI, TORCH_ANCHOR_VERTICAL_OFFSET_LO,
	TORCH_BRANCH_BASE_RADIUS_FRACTION_OF_STALK, TORCH_BRANCH_DEPTH,
	TORCH_BRANCH_RADIUS_CHILD_SCALE_HI, TORCH_BRANCH_RADIUS_CHILD_SCALE_LO, TORCH_CHILD_COUNT_MAX,
	TORCH_CHILD_COUNT_MIN, TORCH_LAST_RING_UNIT_HEIGHT, TORCH_LEAF_RADIUS_FRACTION,
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
pub struct PenmarchTorchScale {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TREE_HEIGHT))]
	pub tree_height: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT_FRACTION))]
	pub stalk_height_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for PenmarchTorchScale {
	fn default() -> Self {
		Self {
			tree_height: DEFAULT_TREE_HEIGHT,
			stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_base_radius: None,
		}
	}
}

impl PenmarchTorchScale {
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
pub struct PenmarchTorchRingParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.20..1.0",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value = "0.1142857"))]
	pub spacing: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub anchors_per_ring: u32,
}

impl Default for PenmarchTorchRingParams {
	fn default() -> Self {
		Self {
			height_range: UnitRange::new(
				DEFAULT_FIRST_RING_UNIT_HEIGHT,
				TORCH_LAST_RING_UNIT_HEIGHT,
			),
			spacing: torch_ring_spacing_unit_height(DEFAULT_STALK_HEIGHT_FRACTION),
			anchors_per_ring: TORCH_ANCHORS_PER_RING,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PenmarchTorchProjectionParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.10..0.45",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub span_fraction_of_height: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_VASE_PROFILE_EPSILON))]
	pub vase_profile_epsilon: f32,
}

impl PenmarchTorchProjectionParams {
	pub fn min_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.min(self.span_fraction_of_height.end)
	}

	pub fn max_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.max(self.span_fraction_of_height.end)
	}
}

impl Default for PenmarchTorchProjectionParams {
	fn default() -> Self {
		Self {
			span_fraction_of_height: UnitRange::new(
				DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
				DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			),
			vase_profile_epsilon: DEFAULT_VASE_PROFILE_EPSILON,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PenmarchTorchGrowthParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_BRANCH_DEPTH))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES))]
	pub angle_tolerance_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_FLARE_ELEVATION_DEGREES))]
	pub flare_elevation_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_SHOULDER_ELEVATION_DEGREES))]
	pub shoulder_elevation_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_CROWN_ELEVATION_DEGREES))]
	pub crown_elevation_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_CROWN_FLIP_RING_U))]
	pub crown_flip_ring_u: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_APEX_ONLY_FLIP_FRACTION, help_heading = "Growth"
	))]
	pub apex_only_flip_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_CHILD_COUNT_MIN))]
	pub child_count_min: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_CHILD_COUNT_MAX))]
	pub child_count_max: u32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = TORCH_BRANCH_BASE_RADIUS_FRACTION_OF_STALK)
	)]
	pub branch_base_radius_fraction_of_stalk: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_BRANCH_RADIUS_CHILD_SCALE_LO))]
	pub branch_radius_child_scale_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_BRANCH_RADIUS_CHILD_SCALE_HI))]
	pub branch_radius_child_scale_hi: f32,
}

impl Default for PenmarchTorchGrowthParams {
	fn default() -> Self {
		Self {
			branch_depth: TORCH_BRANCH_DEPTH,
			angle_tolerance_degrees: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES,
			flare_elevation_degrees: DEFAULT_FLARE_ELEVATION_DEGREES,
			shoulder_elevation_degrees: DEFAULT_SHOULDER_ELEVATION_DEGREES,
			crown_elevation_degrees: DEFAULT_CROWN_ELEVATION_DEGREES,
			crown_flip_ring_u: DEFAULT_CROWN_FLIP_RING_U,
			apex_only_flip_fraction: DEFAULT_APEX_ONLY_FLIP_FRACTION,
			child_count_min: TORCH_CHILD_COUNT_MIN,
			child_count_max: TORCH_CHILD_COUNT_MAX,
			branch_base_radius_fraction_of_stalk: TORCH_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale_lo: TORCH_BRANCH_RADIUS_CHILD_SCALE_LO,
			branch_radius_child_scale_hi: TORCH_BRANCH_RADIUS_CHILD_SCALE_HI,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PenmarchTorchCanopyParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = TORCH_LEAF_RADIUS_FRACTION))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION)
	)]
	pub outer_foliage_distance_fraction: f32,
}

impl Default for PenmarchTorchCanopyParams {
	fn default() -> Self {
		Self {
			leaf_radius_fraction: TORCH_LEAF_RADIUS_FRACTION,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct PenmarchTorchAnchorPerturbationParams {
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

impl Default for PenmarchTorchAnchorPerturbationParams {
	fn default() -> Self {
		Self {
			vertical_offset: UnitRange::new(
				TORCH_ANCHOR_VERTICAL_OFFSET_LO,
				TORCH_ANCHOR_VERTICAL_OFFSET_HI,
			),
			angular_scale: UnitRange::new(
				TORCH_ANCHOR_ANGULAR_SCALE_LO,
				TORCH_ANCHOR_ANGULAR_SCALE_HI,
			),
			radius_offset: UnitRange::new(
				TORCH_ANCHOR_RADIUS_OFFSET_LO,
				TORCH_ANCHOR_RADIUS_OFFSET_HI,
			),
			noise: NoiseParams::default(),
		}
	}
}

impl PenmarchTorchAnchorPerturbationParams {
	pub fn to_perturbation(&self) -> PenmarchTorchAnchorPerturbation {
		PenmarchTorchAnchorPerturbation {
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
pub struct PenmarchTorchSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: PenmarchTorchScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Rings"))]
	pub rings: PenmarchTorchRingParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: PenmarchTorchProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: PenmarchTorchGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy"))]
	pub canopy: PenmarchTorchCanopyParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: PenmarchTorchAnchorPerturbationParams,
}

impl Default for PenmarchTorchSbs {
	fn default() -> Self {
		Self {
			scale: PenmarchTorchScale::default(),
			rings: PenmarchTorchRingParams::default(),
			projection: PenmarchTorchProjectionParams::default(),
			growth: PenmarchTorchGrowthParams::default(),
			canopy: PenmarchTorchCanopyParams::default(),
			canopy_noise: NoiseParams::default(),
			anchor_perturbation: PenmarchTorchAnchorPerturbationParams::default(),
		}
	}
}

impl PenmarchTorchSbs {
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

	pub fn to_proto(&self) -> PenmarchTorchProtoAnchors {
		PenmarchTorchProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			projection_min_fraction_of_height: self.projection.min_fraction(),
			projection_max_fraction_of_height: self.projection.max_fraction(),
			vase_profile_epsilon: self.projection.vase_profile_epsilon,
			projection_center_fraction: DEFAULT_PROJECTION_CENTER_FRACTION,
			flare_elevation_degrees: self.growth.flare_elevation_degrees,
			shoulder_elevation_degrees: self.growth.shoulder_elevation_degrees,
			crown_elevation_degrees: self.growth.crown_elevation_degrees,
			crown_flip_ring_u: self.growth.crown_flip_ring_u,
			apex_only_flip_fraction: self.growth.apex_only_flip_fraction,
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			branch_depth: self.growth.branch_depth,
			child_count_min: self.growth.child_count_min,
			child_count_max: self.growth.child_count_max.max(self.growth.child_count_min),
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
			..PenmarchTorchProtoAnchors::default()
		}
	}

	pub fn to_anchors(&self) -> PenmarchTorchAnchors {
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
		PenmarchTorchAnchors::new(self.to_proto()).with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<StorybookTreeChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for PenmarchTorchSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_frontend_converts_to_proto() -> Result<()> {
		let sbs = PenmarchTorchSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.tree_height - DEFAULT_TREE_HEIGHT).abs() < 1e-5);
		assert!(
			(proto.stalk.stalk_height - DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION).abs()
				< 1e-4
		);
		assert_eq!(proto.branch_depth, TORCH_BRANCH_DEPTH);
		assert_eq!(proto.anchors_per_ring, TORCH_ANCHORS_PER_RING);
		assert!(
			(proto.projection_max_fraction_of_height - DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT)
				.abs() < 1e-5
		);
		Ok(())
	}

	#[test]
	fn build_chain_produces_substantial_graph() {
		let chain = PenmarchTorchSbs::default().build_chain();
		assert!(chain.nodes.len() > 20, "nodes {}", chain.nodes.len());
	}
}
