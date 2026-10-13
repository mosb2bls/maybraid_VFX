//! Shared VegetationComponents LOD emission for jungle canopies (Honu / Jungle Storybook).
//!
//! Classification and proxy construction stay per-tree; this module owns banding + the
//! High / Medium / Low emit loop driven by [`JungleCanopyLodPlan`].

use bevy::prelude::*;
use sbs_geometry::{sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands};
use vegetation_components::{FoliageNode, Placement};

use crate::jungle_growth_vc::{jungle_growth_foliage_nodes, JungleGrowthVcParams};

/// Jitter half-width around the authored growth radius scale.
pub(crate) const RADIUS_SCALE_SPAN: f32 = 0.50;
/// Jitter half-width around the per-tree foliage scale center.
pub(crate) const FOLIAGE_SCALE_SPAN: f32 = 0.40;

/// Site selected for either jungle growth or a cheap canopy ball.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JungleFoliageCandidate {
	pub position: Vec3,
	pub node_idx: usize,
	pub leaf_radius: f32,
}

/// How High / Medium / Low should emit jungle-growth fronds.
#[derive(Clone, Copy, Debug)]
pub(crate) enum JungleGrowthEmitMode {
	/// Every growth candidate (High / Medium).
	All,
	/// Band-sampled growth (kept for callers that still want Medium thinning).
	#[allow(dead_code)]
	Banded(AzimuthHeightBands),
	/// Skip growth fronds (Low); candidates may still feed a proxy AABB.
	None,
}

/// LOD table entry: growth mode, canopy bands, and whether to append a proxy node.
#[derive(Clone, Copy, Debug)]
pub(crate) struct JungleCanopyLodPlan {
	pub growth: JungleGrowthEmitMode,
	pub canopy_bands: AzimuthHeightBands,
	pub include_proxy: bool,
}

/// Authored scales for [`JungleGrowthVcParams::from_node`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct JungleGrowthScaleParams {
	pub radius_scale: f32,
	pub foliage_scale_center: f32,
}

impl JungleGrowthScaleParams {
	pub fn growth_params(self, node_idx: usize, position: Vec3) -> JungleGrowthVcParams {
		JungleGrowthVcParams::from_node(
			node_idx,
			position,
			self.radius_scale,
			RADIUS_SCALE_SPAN,
			self.foliage_scale_center,
			FOLIAGE_SCALE_SPAN,
		)
	}
}

pub(crate) fn banded_jungle_candidates(
	candidates: &[JungleFoliageCandidate],
	bands: AzimuthHeightBands,
) -> Vec<&JungleFoliageCandidate> {
	sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands)
		.into_iter()
		.map(|s| s.item)
		.collect()
}

pub(crate) fn emit_cheap_canopy_ball(position: Vec3, leaf_radius: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(position, leaf_radius))
}

/// Emit growth fronds + banded cheap canopy balls, optionally appending `proxy`.
///
/// `proxy` is only appended when [`JungleCanopyLodPlan::include_proxy`] is set; callers
/// build tree-specific proxies (Honu mid-crown vs Storybook full AABB).
pub(crate) fn emit_jungle_canopy_lod(
	growth: &[JungleFoliageCandidate],
	canopy: &[JungleFoliageCandidate],
	plan: JungleCanopyLodPlan,
	scale: JungleGrowthScaleParams,
	proxy: Option<FoliageNode>,
) -> Vec<FoliageNode> {
	let mut nodes = Vec::new();

	match plan.growth {
		JungleGrowthEmitMode::All => {
			for c in growth {
				nodes.extend(jungle_growth_foliage_nodes(
					scale.growth_params(c.node_idx, c.position),
				));
			}
		}
		JungleGrowthEmitMode::Banded(bands) => {
			for c in banded_jungle_candidates(growth, bands) {
				nodes.extend(jungle_growth_foliage_nodes(
					scale.growth_params(c.node_idx, c.position),
				));
			}
		}
		JungleGrowthEmitMode::None => {}
	}

	for c in banded_jungle_candidates(canopy, plan.canopy_bands) {
		nodes.push(emit_cheap_canopy_ball(c.position, c.leaf_radius));
	}

	if plan.include_proxy {
		if let Some(proxy) = proxy {
			nodes.push(proxy);
		}
	}

	nodes
}
