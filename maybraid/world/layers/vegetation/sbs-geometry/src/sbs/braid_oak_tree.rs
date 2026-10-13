//! **Braid Oak Tree** SBS frontend ([#234](https://github.com/ramate-io/maybraid/issues/234)).
//!
//! Wraps [`StorybookTreeSbs`] with braid art-direction defaults. Flattened clap still exposes storybook
//! field defaults; call [`BraidOakTreeSbs::apply_braid_preset`] after CLI parse.

use std::ops::{Deref, DerefMut};

use procedural_common::UnitRange;

use crate::anchors::braid_oak::{
	BraidOakTreeAnchors, BraidOakTreeProtoAnchors, BRAID_ANCHORS_PER_RING_MAX,
	BRAID_ANGLE_TOLERANCE_DEGREES, BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK, BRAID_BRANCH_DEPTH,
	BRAID_BRANCH_RADIUS_CHILD_SCALE_HI, BRAID_BRANCH_RADIUS_CHILD_SCALE_LO, BRAID_CHILD_COUNT,
	BRAID_FIRST_RING_UNIT_HEIGHT, BRAID_LEAF_RADIUS_FRACTION, BRAID_MAX_PROJECTION_FRACTION,
	BRAID_PROJECTION_MIN_FRACTION, BRAID_RING_SPACING_UNIT_HEIGHT,
	BRAID_STALK_BASE_RADIUS_FRACTION, BRAID_STALK_HEIGHT_FRACTION,
};
use crate::anchors::storybook_tree::{
	DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION, DEFAULT_TREE_HEIGHT,
};
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::scale::{stalk_radius_scaled_range, stalk_scaled_range};
use crate::sbs::storybook_tree::{
	apply_storybook_field_preset, apply_unit_range_preset, StorybookCanopyParams,
	StorybookGrowthParams, StorybookProjectionParams, StorybookRingParams, StorybookTreeSbs,
	StorybookTreeScale,
};
use crate::{BallStickChain, StorybookTreeChain};

fn braid_oak_fields(h: f32) -> StorybookTreeSbs {
	StorybookTreeSbs {
		scale: StorybookTreeScale {
			tree_height: h,
			stalk_height_fraction: BRAID_STALK_HEIGHT_FRACTION,
			stalk_base_radius: Some(BRAID_STALK_BASE_RADIUS_FRACTION * h),
			..StorybookTreeScale::default()
		},
		rings: StorybookRingParams {
			height_range: UnitRange::new(BRAID_FIRST_RING_UNIT_HEIGHT, 1.0),
			spacing: BRAID_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: BRAID_ANCHORS_PER_RING_MAX,
		},
		projection: StorybookProjectionParams {
			span_fraction_of_height: UnitRange::new(
				BRAID_PROJECTION_MIN_FRACTION,
				BRAID_MAX_PROJECTION_FRACTION,
			),
		},
		growth: StorybookGrowthParams {
			branch_depth: BRAID_BRANCH_DEPTH,
			angle_tolerance_degrees: BRAID_ANGLE_TOLERANCE_DEGREES,
			ring_tilt_degrees: 0.0,
			child_count_min: BRAID_CHILD_COUNT,
			child_count_max: BRAID_CHILD_COUNT,
			branch_base_radius_fraction_of_stalk: BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale_lo: BRAID_BRANCH_RADIUS_CHILD_SCALE_LO,
			branch_radius_child_scale_hi: BRAID_BRANCH_RADIUS_CHILD_SCALE_HI,
			..StorybookGrowthParams::default()
		},
		canopy: StorybookCanopyParams {
			leaf_radius_fraction: BRAID_LEAF_RADIUS_FRACTION,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			..StorybookCanopyParams::default()
		},
		..StorybookTreeSbs::default()
	}
}

/// Flattened [`StorybookTreeSbs`] with braid preset constants.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct BraidOakTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub storybook: StorybookTreeSbs,
}

impl Default for BraidOakTreeSbs {
	fn default() -> Self {
		Self { storybook: braid_oak_fields(crate::anchors::storybook_tree::DEFAULT_TREE_HEIGHT) }
	}
}

fn apply_braid_ring_preset(
	current: &mut StorybookRingParams,
	story: &StorybookRingParams,
	braid: &StorybookRingParams,
) {
	apply_unit_range_preset(&mut current.height_range, &story.height_range, &braid.height_range);
	apply_storybook_field_preset(&mut current.spacing, &story.spacing, &braid.spacing);
	apply_storybook_field_preset(
		&mut current.anchors_per_ring,
		&story.anchors_per_ring,
		&braid.anchors_per_ring,
	);
}

impl Deref for BraidOakTreeSbs {
	type Target = StorybookTreeSbs;

	fn deref(&self) -> &Self::Target {
		&self.storybook
	}
}

impl DerefMut for BraidOakTreeSbs {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.storybook
	}
}

impl BraidOakTreeSbs {
	/// Reapply braid preset after flattened clap parse.
	pub fn apply_braid_preset(&mut self) {
		let braid = Self::default().storybook;
		let story = StorybookTreeSbs::default();
		let s = &mut self.storybook;
		let h = s.scale.tree_height.max(1e-6);

		apply_storybook_field_preset(
			&mut s.scale.stalk_height_fraction,
			&story.scale.stalk_height_fraction,
			&braid.scale.stalk_height_fraction,
		);

		let story_stalk = crate::anchors::storybook_tree::DEFAULT_STALK_BASE_RADIUS_FRACTION * h;
		let braid_stalk = BRAID_STALK_BASE_RADIUS_FRACTION * h;
		match s.scale.stalk_base_radius {
			None => s.scale.stalk_base_radius = Some(braid_stalk),
			Some(r) if (r - story_stalk).abs() < 1e-4 => {
				s.scale.stalk_base_radius = Some(braid_stalk);
			}
			_ => {}
		}

		apply_braid_ring_preset(&mut s.rings, &story.rings, &braid.rings);
		apply_unit_range_preset(
			&mut s.projection.span_fraction_of_height,
			&story.projection.span_fraction_of_height,
			&braid.projection.span_fraction_of_height,
		);
		apply_storybook_field_preset(&mut s.growth, &story.growth, &braid.growth);
		apply_storybook_field_preset(&mut s.canopy, &story.canopy, &braid.canopy);
	}

	pub fn height(&self) -> f32 {
		self.storybook.height()
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.storybook.leaf_radius_world()
	}

	pub fn to_proto(&self) -> BraidOakTreeProtoAnchors {
		BraidOakTreeProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			max_projection_fraction_of_height: self.projection.max_fraction(),
			projection_min_fraction_of_height: self.projection.min_fraction(),
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			bias_blend: crate::anchors::braid_oak::BRAID_BIAS_BLEND,
			branch_depth: self.growth.branch_depth,
			child_count_min: self.growth.child_count_min,
			child_count_max: self.growth.child_count_max.max(self.growth.child_count_min),
			outer_foliage_distance_fraction: self.canopy.outer_foliage_distance_fraction,
			branch_base_radius_fraction_of_stalk: self.growth.branch_base_radius_fraction_of_stalk,
			branch_radius_child_scale: (
				self.growth.branch_radius_child_scale_lo,
				self.growth.branch_radius_child_scale_hi,
			),
		}
	}

	pub fn to_anchors(&self) -> BraidOakTreeAnchors {
		let mut perturbation = self.anchor_perturbation.to_perturbation();
		let scaled = stalk_scaled_range(
			UnitRange::new(perturbation.vertical_offset.start, perturbation.vertical_offset.end),
			self.scale.stalk_height(),
			DEFAULT_TREE_HEIGHT * BRAID_STALK_HEIGHT_FRACTION,
		);
		perturbation.vertical_offset = scaled.start..scaled.end;
		let radius_scaled = stalk_radius_scaled_range(
			UnitRange::new(perturbation.radius_offset.start, perturbation.radius_offset.end),
			self.scale.stalk_base_radius_or_default(),
			DEFAULT_TREE_HEIGHT * BRAID_STALK_BASE_RADIUS_FRACTION,
		);
		perturbation.radius_offset = radius_scaled.start..radius_scaled.end;
		BraidOakTreeAnchors::new(self.to_proto()).with_perturbation(perturbation)
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for BraidOakTreeSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		let noise = procedural_common::NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::braid_oak::{BRAID_MAX_PROJECTION_FRACTION, BRAID_PROJECTION_MIN_FRACTION};
	use anyhow::Result;

	#[test]
	fn default_builds_chain() -> Result<()> {
		let chain = BraidOakTreeSbs::default().build_chain();
		assert!(chain.nodes.len() > 20, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn apply_braid_preset_after_storybook_cli_defaults() -> Result<()> {
		let mut geometry = BraidOakTreeSbs { storybook: StorybookTreeSbs::default() };
		geometry.apply_braid_preset();
		let proto = geometry.to_proto();
		assert!(
			(proto.stalk.stalk_height - geometry.height() * BRAID_STALK_HEIGHT_FRACTION).abs()
				< 1e-3
		);
		assert!((proto.first_ring_unit_height - BRAID_FIRST_RING_UNIT_HEIGHT).abs() < 1e-4);
		assert!(
			(proto.max_projection_fraction_of_height - BRAID_MAX_PROJECTION_FRACTION).abs() < 1e-4
		);
		assert!(
			(proto.projection_min_fraction_of_height - BRAID_PROJECTION_MIN_FRACTION).abs() < 1e-4
		);
		assert_eq!(proto.child_count_min, BRAID_CHILD_COUNT);
		assert_eq!(proto.child_count_max, BRAID_CHILD_COUNT);
		assert_eq!(proto.branch_depth, BRAID_BRANCH_DEPTH);
		Ok(())
	}

	#[test]
	fn leaf_radius_scales_with_tree_height() -> Result<()> {
		let sbs = BraidOakTreeSbs::default();
		assert!((sbs.leaf_radius_world() - BRAID_LEAF_RADIUS_FRACTION * sbs.height()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn unit_height_scales_anchor_perturbation() -> Result<()> {
		let mut geometry = BraidOakTreeSbs::default();
		geometry.scale.tree_height = 1.0;
		geometry.scale.stalk_base_radius = Some(BRAID_STALK_BASE_RADIUS_FRACTION);
		let anchors = geometry.to_anchors();
		let v = anchors.perturbation.vertical_offset;
		assert!(
			v.start.abs() < 0.12 && v.end.abs() < 0.12,
			"unit oak vertical perturbation should shrink with stalk, got {v:?}"
		);
		Ok(())
	}
}
