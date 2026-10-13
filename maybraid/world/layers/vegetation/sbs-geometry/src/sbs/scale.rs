//! Helpers for keeping SBS silhouettes readable across tree heights.

use std::ops::Range;

use procedural_common::UnitRange;

/// Ratio of this construction's stalk height to the construction's authored full-size default.
pub fn stalk_scale_ratio(stalk_height: f32, reference_stalk_height: f32) -> f32 {
	(stalk_height.max(1e-6) / reference_stalk_height.max(1e-6)).clamp(0.0, 1.0)
}

/// Scale a full-size perturbation range down with stalk height.
pub fn stalk_scaled_range(
	range: UnitRange,
	stalk_height: f32,
	reference_stalk_height: f32,
) -> UnitRange {
	let scale = stalk_scale_ratio(stalk_height, reference_stalk_height);
	UnitRange::new(range.start * scale, range.end * scale)
}

/// Scale a full-size radius perturbation range down with stalk base radius.
pub fn stalk_radius_scaled_range(
	range: UnitRange,
	stalk_base_radius: f32,
	reference_stalk_base_radius: f32,
) -> UnitRange {
	let base = stalk_base_radius.max(1e-6);
	let scale = (base / reference_stalk_base_radius.max(1e-6)).clamp(0.0, 1.0);
	let max_abs = base * 0.006;
	UnitRange::new(
		(range.start * scale).clamp(-max_abs, max_abs),
		(range.end * scale).clamp(-max_abs, max_abs),
	)
}

/// Smaller forms need larger foliage relative to height to stay legible.
pub fn leaf_radius_for_stalk_scale(
	tree_height: f32,
	leaf_radius_fraction: f32,
	stalk_height: f32,
	reference_stalk_height: f32,
	reference_tree_height: f32,
) -> f32 {
	let stalk_ratio = stalk_scale_ratio(stalk_height, reference_stalk_height);
	let tree_ratio = stalk_scale_ratio(tree_height, reference_tree_height);
	let ratio = stalk_ratio.max(tree_ratio).max(0.02);
	let boost = ratio.powf(-0.18).clamp(1.0, 1.55);
	tree_height.max(1e-6) * leaf_radius_fraction * boost
}

/// Keep branch bases from hitting full-size hard floors on mini trees.
pub fn branch_radius_floor(stalk_base_radius: f32, full_size_floor: f32) -> f32 {
	(stalk_base_radius.max(1e-6) * 0.04).clamp(0.0015, full_size_floor)
}

/// Clamp branch seed radii against stalk-relative caps so mini limbs do not start as trunk-sized.
pub fn branch_base_radius_for_stalk(
	stalk_base_radius: f32,
	branch_base_radius_fraction_of_stalk: f32,
	full_size_floor: f32,
	stalk_height: f32,
	reference_stalk_height: f32,
) -> f32 {
	let base = stalk_base_radius.max(1e-6);
	let raw = base * branch_base_radius_fraction_of_stalk;
	let scale = stalk_scale_ratio(stalk_height, reference_stalk_height);
	let max_fraction_of_stalk = 0.35 + scale * 0.25;
	raw.max(branch_radius_floor(base, full_size_floor))
		.min(base * max_fraction_of_stalk)
}

/// Absolute child radius bounds carried through [`crate::BranchOut`] propagation.
pub fn branch_radius_child_bounds_for_stalk(
	stalk_base_radius: f32,
	branch_base_radius: f32,
	stalk_height: f32,
	reference_stalk_height: f32,
) -> Range<f32> {
	let base = stalk_base_radius.max(1e-6);
	let scale = stalk_scale_ratio(stalk_height, reference_stalk_height);
	let min_fraction_of_stalk = 0.08 + (1.0 - scale) * 0.02;
	let min = (base * min_fraction_of_stalk).clamp(0.0015, branch_base_radius.max(0.0015));
	min..branch_base_radius.max(min)
}

/// Thin child branches slightly faster on smaller stalks so angle bias and silhouette remain visible.
pub fn branch_radius_child_scale_for_stalk(
	scale: (f32, f32),
	stalk_height: f32,
	reference_stalk_height: f32,
) -> (f32, f32) {
	let ratio = stalk_scale_ratio(stalk_height, reference_stalk_height);
	let reduction = (1.0 - ratio).clamp(0.0, 1.0) * 0.12;
	((scale.0 - reduction).max(0.55), (scale.1 - reduction).max(0.60))
}

/// Delay terminal foliage on mini trees so the underlying projection direction remains readable.
pub fn outer_foliage_distance_for_stalk(
	base: f32,
	stalk_height: f32,
	reference_stalk_height: f32,
) -> f32 {
	let ratio = stalk_scale_ratio(stalk_height, reference_stalk_height);
	(base + (1.0 - ratio) * 0.12).clamp(base, 0.82)
}

#[cfg(test)]
mod tests {
	use crate::anchors::rorys_head_trained::DEFAULT_TREE_HEIGHT as RORY_DEFAULT_TREE_HEIGHT;
	use crate::anchors::storybook_tree::DEFAULT_TREE_HEIGHT as STORYBOOK_DEFAULT_TREE_HEIGHT;
	use crate::anchors::vase_tree::DEFAULT_TREE_HEIGHT as VASE_DEFAULT_TREE_HEIGHT;
	use crate::sbs::kamakura_torch::KamakuraTorchSbs;
	use crate::sbs::penmarch_torch::PenmarchTorchSbs;
	use crate::sbs::rorys_head_trained::RorysHeadTrainedSbs;
	use crate::sbs::storybook_tree::StorybookTreeSbs;
	use crate::sbs::vase_tree::VaseTreeSbs;
	use anyhow::Result;
	use procedural_common::{NoiseConfig, NoiseParams};

	fn first_branch_radius(seeds: &[crate::StorybookTreeChain]) -> Option<f32> {
		seeds
			.iter()
			.find_map(|seed| seed.active_branch_profile().map(|branch| branch.node.radius))
	}

	fn first_branch_bias_y(seeds: &[crate::StorybookTreeChain]) -> Option<f32> {
		seeds
			.iter()
			.find_map(|seed| seed.active_branch_profile().map(|branch| branch.bias_ray.y))
	}

	fn built_branch_radii(chain: &crate::BallStickChain<crate::StorybookTreeChain>) -> Vec<f32> {
		chain
			.nodes_with_hysteresis()
			.filter_map(|(_node, h)| h.active_branch_profile().map(|branch| branch.node.radius))
			.collect()
	}

	#[test]
	fn mini_leaf_radius_gets_relative_boost_without_changing_full_size_defaults() -> Result<()> {
		let full_story = StorybookTreeSbs::default();
		assert!((full_story.leaf_radius_world() - full_story.height() * 0.09).abs() < 1e-4);

		let mut mini_story = full_story.clone();
		mini_story.scale.tree_height = 1.8;
		assert!(
			mini_story.leaf_radius_world() / mini_story.height()
				> full_story.leaf_radius_world() / full_story.height()
		);

		let mut mini_vase = VaseTreeSbs::default();
		let full_vase = mini_vase.clone();
		mini_vase.scale.tree_height = 1.8;
		assert!(
			mini_vase.leaf_radius_world() / mini_vase.height()
				> full_vase.leaf_radius_world() / full_vase.height()
		);

		let mut mini_rory = RorysHeadTrainedSbs::default();
		let full_rory = mini_rory.clone();
		mini_rory.scale.tree_height = 1.8;
		assert!(
			mini_rory.leaf_radius_world() / mini_rory.height()
				> full_rory.leaf_radius_world() / full_rory.height()
		);

		let mut mini_torch = PenmarchTorchSbs::default();
		let full_torch = mini_torch.clone();
		mini_torch.scale.tree_height = 1.8;
		assert!(
			mini_torch.leaf_radius_world() / mini_torch.height()
				> full_torch.leaf_radius_world() / full_torch.height()
		);
		Ok(())
	}

	#[test]
	fn mini_vertical_perturbation_scales_with_stalk_height() -> Result<()> {
		let mut mini = StorybookTreeSbs::default();
		mini.scale.tree_height = 1.8;
		let anchors = mini.to_anchors();
		let scale = mini.scale.stalk_height()
			/ (STORYBOOK_DEFAULT_TREE_HEIGHT * mini.scale.stalk_height_fraction);
		assert!((anchors.perturbation.vertical_offset.start + scale).abs() < 1e-4);
		assert!((anchors.perturbation.vertical_offset.end - scale).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn mini_branch_radius_floor_tracks_stalk_radius() -> Result<()> {
		let mut mini_vase = VaseTreeSbs::default();
		mini_vase.scale.tree_height = 1.8;
		mini_vase.scale.stalk_base_radius = Some(0.03);
		let mini_seeds =
			mini_vase.to_proto().hysteresis_seeds(NoiseConfig::new(NoiseParams::default()));
		let mini_radius = first_branch_radius(&mini_seeds)
			.ok_or_else(|| anyhow::anyhow!("expected a mini vase branch seed"))?;

		let full_vase = VaseTreeSbs::default();
		let full_seeds =
			full_vase.to_proto().hysteresis_seeds(NoiseConfig::new(NoiseParams::default()));
		let full_radius = first_branch_radius(&full_seeds)
			.ok_or_else(|| anyhow::anyhow!("expected a full vase branch seed"))?;

		assert!(mini_radius < 0.01, "mini radius {mini_radius}");
		assert!(full_radius > mini_radius * 5.0, "full {full_radius}, mini {mini_radius}");
		assert!(
			(mini_radius / mini_vase.scale.stalk_base_radius_or_default()) > 0.10,
			"mini radius should remain readable relative to stalk"
		);
		Ok(())
	}

	#[test]
	fn mini_branch_radii_stay_within_stalk_relative_bounds() -> Result<()> {
		let mut mini_vase = VaseTreeSbs::default();
		mini_vase.scale.tree_height = 1.8;
		mini_vase.scale.stalk_base_radius = Some(0.03);
		let vase_radii = built_branch_radii(&mini_vase.build_chain());
		let vase_min = vase_radii.iter().copied().fold(f32::INFINITY, f32::min);
		let vase_max = vase_radii.iter().copied().fold(0.0_f32, f32::max);
		let vase_stalk = mini_vase.scale.stalk_base_radius_or_default();
		assert!(vase_min >= vase_stalk * 0.08, "vase min {vase_min}");
		assert!(vase_max <= vase_stalk * 0.36, "vase max {vase_max}");

		let mut mini_rory = RorysHeadTrainedSbs::default();
		mini_rory.scale.tree_height = 1.6;
		mini_rory.scale.stalk_base_radius = Some(0.024);
		let rory_radii = built_branch_radii(&mini_rory.build_chain());
		let rory_min = rory_radii.iter().copied().fold(f32::INFINITY, f32::min);
		let rory_max = rory_radii.iter().copied().fold(0.0_f32, f32::max);
		let rory_stalk = mini_rory.scale.stalk_base_radius_or_default();
		assert!(rory_min >= rory_stalk * 0.08, "rory min {rory_min}");
		assert!(rory_max <= rory_stalk * 0.38, "rory max {rory_max}");
		Ok(())
	}

	#[test]
	fn small_storybook_stays_flatter_than_vase_and_torch() -> Result<()> {
		let mut story = StorybookTreeSbs::default();
		story.scale.tree_height = 1.8;
		let story_seeds = story.hysteresis_seeds();
		let story_y = first_branch_bias_y(&story_seeds)
			.ok_or_else(|| anyhow::anyhow!("expected storybook branch seed"))?
			.abs();

		let mut vase = VaseTreeSbs::default();
		vase.scale.tree_height = 1.8;
		let vase_seeds = vase.hysteresis_seeds();
		let vase_y = first_branch_bias_y(&vase_seeds)
			.ok_or_else(|| anyhow::anyhow!("expected vase branch seed"))?;

		let mut torch = KamakuraTorchSbs::default();
		torch.scale.tree_height = 1.8;
		let torch_seeds = torch.hysteresis_seeds();
		let torch_y = first_branch_bias_y(&torch_seeds)
			.ok_or_else(|| anyhow::anyhow!("expected torch branch seed"))?;

		assert!(story_y < 0.10, "storybook y {story_y}");
		assert!(vase_y > story_y + 0.35, "vase y {vase_y}, story y {story_y}");
		assert!(torch_y > vase_y, "torch y {torch_y}, vase y {vase_y}");
		Ok(())
	}

	#[test]
	fn reference_heights_are_available_for_all_normalized_frontends() -> Result<()> {
		assert!(STORYBOOK_DEFAULT_TREE_HEIGHT > 0.0);
		assert!(VASE_DEFAULT_TREE_HEIGHT > 0.0);
		assert!(RORY_DEFAULT_TREE_HEIGHT > 0.0);
		Ok(())
	}
}
