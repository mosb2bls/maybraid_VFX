//! Restricted **Storybook Tree** geometry for CLI and playgrounds ([#230](https://github.com/ramate-io/maybraid/issues/230)).

#[cfg(feature = "clap")]
use procedural_common::{noise_params_from_scalar_str, parse_unit_range};
use procedural_common::{NoiseConfig, NoiseParams, UnitRange};

use crate::anchors::storybook_tree::{
	StorybookTreeAnchorPerturbation, StorybookTreeAnchors, StorybookTreeProtoAnchors,
	DEFAULT_FIRST_RING_UNIT_HEIGHT, DEFAULT_MAX_PROJECTION_FRACTION,
	DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION, DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
	DEFAULT_STALK_BASE_RADIUS_FRACTION, DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_TREE_HEIGHT,
};
use crate::anchors::strict_stalk::StrictStalk;
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
pub struct StorybookTreeScale {
	/// Total tree height `H` in world units. Drives stalk dimensions, ring placement, and projection budgets.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_TREE_HEIGHT))]
	pub tree_height: f32,
	/// Stalk length as a fraction of [`Self::tree_height`] (RFC default [`DEFAULT_STALK_HEIGHT_FRACTION`]).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = DEFAULT_STALK_HEIGHT_FRACTION))]
	pub stalk_height_fraction: f32,
	/// Trunk base radius in world units. When `None`, [`Self::stalk_base_radius_or_default`] uses
	/// [`DEFAULT_STALK_BASE_RADIUS_FRACTION`] × [`Self::tree_height`].
	#[cfg_attr(feature = "clap", arg(long))]
	pub stalk_base_radius: Option<f32>,
}

impl Default for StorybookTreeScale {
	fn default() -> Self {
		Self {
			tree_height: DEFAULT_TREE_HEIGHT,
			stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_base_radius: None,
		}
	}
}

impl StorybookTreeScale {
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
pub struct StorybookRingParams {
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "ring-heights",
			default_value = "0.30..1.0",
			value_parser = parse_unit_range,
			value_name = "FIRST..LAST"
		)
	)]
	pub height_range: UnitRange,
	#[cfg_attr(feature = "clap", arg(long, default_value = "0.10"))]
	pub spacing: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 6))]
	pub anchors_per_ring: u32,
}

impl Default for StorybookRingParams {
	fn default() -> Self {
		Self {
			height_range: UnitRange::new(DEFAULT_FIRST_RING_UNIT_HEIGHT, 1.0),
			spacing: 0.10,
			anchors_per_ring: 6,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct StorybookProjectionParams {
	/// End-ring minimum and mid-canopy maximum limb projection as fractions of tree height `H` (`min..max`).
	#[cfg_attr(
		feature = "clap",
		arg(
			long = "projection",
			default_value = "0.20..0.50",
			value_parser = parse_unit_range,
			value_name = "MIN..MAX"
		)
	)]
	pub span_fraction_of_height: UnitRange,
}

impl StorybookProjectionParams {
	pub fn min_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.min(self.span_fraction_of_height.end)
	}

	pub fn max_fraction(&self) -> f32 {
		self.span_fraction_of_height.start.max(self.span_fraction_of_height.end)
	}
}

impl Default for StorybookProjectionParams {
	fn default() -> Self {
		Self {
			span_fraction_of_height: UnitRange::new(
				DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
				DEFAULT_MAX_PROJECTION_FRACTION,
			),
		}
	}
}

/// Overwrite `current` with `preset` when it still matches storybook defaults (CLI merge helper).
pub(crate) fn apply_storybook_field_preset<T: Clone + PartialEq>(
	current: &mut T,
	story: &T,
	preset: &T,
) {
	if *current == *story {
		*current = preset.clone();
	}
}

/// Merge a [`UnitRange`] preset field-by-field when the user did not replace the whole range.
pub(crate) fn apply_unit_range_preset(
	current: &mut UnitRange,
	story: &UnitRange,
	preset: &UnitRange,
) {
	if *current == *story {
		*current = preset.clone();
		return;
	}
	apply_storybook_field_preset(&mut current.start, &story.start, &preset.start);
	apply_storybook_field_preset(&mut current.end, &story.end, &preset.end);
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct StorybookGrowthParams {
	/// Limb segment hops (`3`–`5`); must match a [`segment_fracs`](crate::segment_fracs) table ([`storybook_branch_depth`](crate::chain::storybook_tree::storybook_branch_depth) coerces out-of-range CLI values at anchor build).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4))]
	pub branch_depth: usize,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 33.0))]
	pub angle_tolerance_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4.0))]
	pub ring_tilt_degrees: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1))]
	pub child_count_min: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3))]
	pub child_count_max: u32,
	/// Limb radius at the ring anchor as a fraction of stalk base radius.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.25))]
	pub branch_base_radius_fraction_of_stalk: f32,
	/// Per-hop radius scale range (lo..hi) applied down each limb.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.75))]
	pub branch_radius_child_scale_lo: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.82))]
	pub branch_radius_child_scale_hi: f32,
}

impl Default for StorybookGrowthParams {
	fn default() -> Self {
		Self {
			branch_depth: 4,
			angle_tolerance_degrees: 33.0,
			ring_tilt_degrees: 4.0,
			child_count_min: 1,
			child_count_max: 3,
			branch_base_radius_fraction_of_stalk: 0.25,
			branch_radius_child_scale_lo: 0.75,
			branch_radius_child_scale_hi: 0.82,
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct StorybookCanopyParams {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.09))]
	pub leaf_radius_fraction: f32,
	#[cfg_attr(
		feature = "clap",
		arg(long, default_value_t = DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION)
	)]
	pub outer_foliage_distance_fraction: f32,
}

impl Default for StorybookCanopyParams {
	fn default() -> Self {
		Self {
			leaf_radius_fraction: 0.09,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
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
	pub fn to_perturbation(&self) -> StorybookTreeAnchorPerturbation {
		StorybookTreeAnchorPerturbation {
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
pub struct StorybookTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Scale"))]
	pub scale: StorybookTreeScale,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Rings"))]
	pub rings: StorybookRingParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Projection"))]
	pub projection: StorybookProjectionParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Growth"))]
	pub growth: StorybookGrowthParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy"))]
	pub canopy: StorybookCanopyParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Canopy Noise"))]
	pub canopy_noise: NoiseParams,
	#[cfg_attr(feature = "clap", command(flatten, next_help_heading = "Anchor Perturbation"))]
	pub anchor_perturbation: AnchorPerturbationParams,
}

impl Default for StorybookTreeSbs {
	fn default() -> Self {
		Self {
			scale: StorybookTreeScale::default(),
			rings: StorybookRingParams::default(),
			projection: StorybookProjectionParams::default(),
			growth: StorybookGrowthParams::default(),
			canopy: StorybookCanopyParams::default(),
			canopy_noise: NoiseParams::default(),
			anchor_perturbation: AnchorPerturbationParams::default(),
		}
	}
}

impl StorybookTreeSbs {
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

	pub fn to_proto(&self) -> StorybookTreeProtoAnchors {
		StorybookTreeProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			max_projection_fraction_of_height: self.projection.max_fraction(),
			projection_min_fraction_of_height: self.projection.min_fraction(),
			ring_tilt_degrees: self.growth.ring_tilt_degrees,
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			bias_blend: 0.88,
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
			..StorybookTreeProtoAnchors::default()
		}
	}

	pub fn to_anchors(&self) -> StorybookTreeAnchors {
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
		StorybookTreeAnchors::new(self.to_proto()).with_perturbation(perturbation)
	}

	pub fn hysteresis_seeds(&self) -> Vec<StorybookTreeChain> {
		let noise = NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for StorybookTreeSbs {
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
		let sbs = StorybookTreeSbs::default();
		let proto = sbs.to_proto();
		assert!((proto.tree_height - DEFAULT_TREE_HEIGHT).abs() < 1e-5);
		assert!(
			(proto.stalk.stalk_height - DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION).abs()
				< 1e-4
		);
		assert_eq!(proto.branch_depth, 4);
		assert_eq!(proto.anchors_per_ring, 6);
		assert!(
			(proto.first_ring_unit_height - DEFAULT_FIRST_RING_UNIT_HEIGHT).abs() < 1e-5,
			"first ring {}",
			proto.first_ring_unit_height
		);
		Ok(())
	}

	#[test]
	fn build_chain_produces_substantial_graph() -> Result<()> {
		let sbs = StorybookTreeSbs::default();
		let chain = sbs.build_chain();
		assert!(chain.nodes.len() > 50, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn projection_span_defaults_match_legacy_dome() -> Result<()> {
		let sbs = StorybookTreeSbs::default();
		assert!((sbs.projection.max_fraction() - DEFAULT_MAX_PROJECTION_FRACTION).abs() < 1e-5);
		assert!(
			(sbs.projection.min_fraction() - DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT).abs()
				< 1e-5
		);
		let proto = sbs.to_proto();
		let h = proto.tree_height;
		let l_end = proto.projection_length(proto.ring_mix_u(proto.first_ring_unit_height));
		assert!((l_end - h * DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn mid_canopy_longer_projection_than_ends() -> Result<()> {
		let proto = StorybookTreeSbs::default().to_proto();
		let l_low = proto.projection_length(proto.ring_mix_u(proto.first_ring_unit_height));
		let l_high = proto.projection_length(proto.ring_mix_u(proto.last_ring_unit_height));
		let l_mid = proto.projection_length(0.5);
		assert!(l_mid > l_low);
		assert!(l_mid > l_high);
		Ok(())
	}

	#[test]
	fn leaf_radius_scales_with_tree_height() -> Result<()> {
		let sbs = StorybookTreeSbs::default();
		assert!((sbs.leaf_radius_world() - 0.09 * sbs.height()).abs() < 1e-4);
		Ok(())
	}
}
