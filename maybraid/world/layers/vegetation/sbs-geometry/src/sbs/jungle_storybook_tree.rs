//! **Jungle Storybook Tree** SBS frontend ([#235](https://github.com/ramate-io/maybraid/issues/235)).
//!
//! Wraps [`StorybookTreeSbs`] with jungle art-direction defaults. Flattened clap still exposes storybook
//! field defaults; call [`JungleStorybookTreeSbs::apply_jungle_preset`] after CLI parse (see
//! [`sbs_trees::jungle_storybook_tree`](../../sbs-trees/src/jungle_storybook_tree.rs)).

use std::ops::{Deref, DerefMut};

use procedural_common::UnitRange;

use crate::anchors::storybook_tree::DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION;
use crate::anchors::Anchors;
use crate::sbs::storybook_tree::{
	apply_storybook_field_preset, apply_unit_range_preset, StorybookCanopyParams,
	StorybookGrowthParams, StorybookProjectionParams, StorybookRingParams, StorybookTreeSbs,
	StorybookTreeScale,
};
use crate::{BallStickChain, StorybookTreeChain};

// -----------------------------------------------------------------------------
// Jungle preset constants (not applied by flattened clap — use `apply_jungle_preset`)
// -----------------------------------------------------------------------------

/// Stalk base radius as a fraction of total height `H`.
pub const JUNGLE_STALK_BASE_RADIUS_FRACTION: f32 = 0.065;

/// Limb radius at ring anchors as a fraction of stalk base radius (storybook: `0.12`).
pub const JUNGLE_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.30;

/// Lowest canopy ring as a unit-height fraction along the stalk (storybook: `0.30`).
pub const JUNGLE_FIRST_RING_UNIT_HEIGHT: f32 = 0.40;

/// Vertical spacing between ring planes as a fraction of stalk height (storybook SBS: `0.10`).
pub const JUNGLE_RING_SPACING_UNIT_HEIGHT: f32 = 0.14;

/// Radial spokes per ring (storybook: `6`).
pub const JUNGLE_ANCHORS_PER_RING: u32 = 5;

/// [`BranchOut`](crate::chain::BranchOut) hops per limb projection (storybook: `4`).
pub const JUNGLE_BRANCH_DEPTH: usize = 5;

/// Max limb projection length as a fraction of `H` (storybook: `0.50`).
pub const JUNGLE_MAX_PROJECTION_FRACTION: f32 = 0.58;

/// Wider branch fan-out than storybook (`26°` default).
pub const JUNGLE_ANGLE_TOLERANCE_DEGREES: f32 = 33.0;

/// World leaf radius for inner-ball / outer-splay canopy (not jungle-growth clusters).
pub const JUNGLE_LEAF_RADIUS_FRACTION: f32 = 0.15;

/// Flattens [`StorybookTreeSbs`] with the constants above.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct JungleStorybookTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub storybook: StorybookTreeSbs,
}

fn jungle_storybook_fields(h: f32) -> StorybookTreeSbs {
	StorybookTreeSbs {
		scale: StorybookTreeScale {
			tree_height: h,
			stalk_height_fraction: crate::anchors::storybook_tree::DEFAULT_STALK_HEIGHT_FRACTION,
			stalk_base_radius: Some(JUNGLE_STALK_BASE_RADIUS_FRACTION * h),
			..StorybookTreeScale::default()
		},
		rings: StorybookRingParams {
			height_range: UnitRange::new(JUNGLE_FIRST_RING_UNIT_HEIGHT, 1.0),
			spacing: JUNGLE_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: JUNGLE_ANCHORS_PER_RING,
		},
		projection: StorybookProjectionParams {
			span_fraction_of_height: UnitRange::new(
				crate::anchors::storybook_tree::DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
				JUNGLE_MAX_PROJECTION_FRACTION,
			),
		},
		growth: StorybookGrowthParams {
			branch_depth: JUNGLE_BRANCH_DEPTH,
			branch_base_radius_fraction_of_stalk: JUNGLE_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			angle_tolerance_degrees: JUNGLE_ANGLE_TOLERANCE_DEGREES,
			child_count_min: 1,
			child_count_max: 2,
			branch_radius_child_scale_lo: 0.72,
			branch_radius_child_scale_hi: 0.80,
			..StorybookGrowthParams::default()
		},
		canopy: StorybookCanopyParams {
			leaf_radius_fraction: JUNGLE_LEAF_RADIUS_FRACTION,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			..StorybookCanopyParams::default()
		},
		..StorybookTreeSbs::default()
	}
}

impl Default for JungleStorybookTreeSbs {
	fn default() -> Self {
		Self {
			storybook: jungle_storybook_fields(crate::anchors::storybook_tree::DEFAULT_TREE_HEIGHT),
		}
	}
}

fn apply_jungle_ring_preset(
	current: &mut StorybookRingParams,
	story: &StorybookRingParams,
	jungle: &StorybookRingParams,
) {
	apply_unit_range_preset(&mut current.height_range, &story.height_range, &jungle.height_range);
	apply_storybook_field_preset(&mut current.spacing, &story.spacing, &jungle.spacing);
	apply_storybook_field_preset(
		&mut current.anchors_per_ring,
		&story.anchors_per_ring,
		&jungle.anchors_per_ring,
	);
}

impl Deref for JungleStorybookTreeSbs {
	type Target = StorybookTreeSbs;

	fn deref(&self) -> &Self::Target {
		&self.storybook
	}
}

impl DerefMut for JungleStorybookTreeSbs {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.storybook
	}
}

impl JungleStorybookTreeSbs {
	/// Reapply jungle preset after flattened clap parse.
	///
	/// Only fields that still equal [`StorybookTreeSbs::default`] are overwritten, so explicit CLI
	/// overrides (e.g. `--tree-height`) are preserved.
	pub fn apply_jungle_preset(&mut self) {
		let jungle = Self::default().storybook;
		let story = StorybookTreeSbs::default();
		let s = &mut self.storybook;
		let h = s.scale.tree_height.max(1e-6);

		let story_stalk = crate::anchors::storybook_tree::DEFAULT_STALK_BASE_RADIUS_FRACTION * h;
		let jungle_stalk = JUNGLE_STALK_BASE_RADIUS_FRACTION * h;
		match s.scale.stalk_base_radius {
			None => s.scale.stalk_base_radius = Some(jungle_stalk),
			Some(r) if (r - story_stalk).abs() < 1e-4 => {
				s.scale.stalk_base_radius = Some(jungle_stalk);
			}
			_ => {}
		}

		apply_jungle_ring_preset(&mut s.rings, &story.rings, &jungle.rings);
		apply_unit_range_preset(
			&mut s.projection.span_fraction_of_height,
			&story.projection.span_fraction_of_height,
			&jungle.projection.span_fraction_of_height,
		);
		apply_storybook_field_preset(&mut s.growth, &story.growth, &jungle.growth);
		apply_storybook_field_preset(&mut s.canopy, &story.canopy, &jungle.canopy);
	}

	pub fn height(&self) -> f32 {
		self.storybook.height()
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.storybook.leaf_radius_world()
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.storybook.build_chain()
	}
}

impl Anchors<StorybookTreeChain> for JungleStorybookTreeSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.storybook.anchors()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn assert_jungle_preset(
		proto: &crate::StorybookTreeProtoAnchors,
		story: &crate::StorybookTreeProtoAnchors,
	) {
		assert!(proto.stalk.stalk_base_radius > story.stalk.stalk_base_radius);
		assert!(
			proto.branch_base_radius_fraction_of_stalk > story.branch_base_radius_fraction_of_stalk
		);
		assert!(proto.first_ring_unit_height > story.first_ring_unit_height);
		assert!(proto.ring_spacing_unit_height > story.ring_spacing_unit_height);
		assert!(proto.anchors_per_ring <= story.anchors_per_ring);
		assert_eq!(proto.branch_depth, JUNGLE_BRANCH_DEPTH);
	}

	#[test]
	fn default_builds_chain() -> Result<()> {
		let chain = JungleStorybookTreeSbs::default().build_chain();
		assert!(chain.nodes.len() > 50, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn apply_jungle_preset_after_storybook_cli_defaults() -> Result<()> {
		let mut geometry = JungleStorybookTreeSbs { storybook: StorybookTreeSbs::default() };
		geometry.apply_jungle_preset();
		let story = StorybookTreeSbs::default().to_proto();
		assert_jungle_preset(&geometry.storybook.to_proto(), &story);
		assert!(
			(geometry.storybook.canopy.leaf_radius_fraction - JUNGLE_LEAF_RADIUS_FRACTION).abs()
				< 1e-6
		);
		Ok(())
	}

	#[test]
	fn jungle_differs_from_storybook() -> Result<()> {
		let jungle = JungleStorybookTreeSbs::default();
		let story = StorybookTreeSbs::default();
		assert_jungle_preset(&jungle.storybook.to_proto(), &story.to_proto());
		assert!(
			(jungle.storybook.canopy.leaf_radius_fraction - JUNGLE_LEAF_RADIUS_FRACTION).abs()
				< 1e-6
		);
		assert!(jungle.build_chain().nodes.len() > 30);
		Ok(())
	}
}
