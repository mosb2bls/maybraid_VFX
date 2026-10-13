//! VegetationComponents emission for jungle growth clusters (palm fronds + spears, no ball mass).
//!
//! Approximates jungle-growth foliage without the inner dirt/wood ball: a short palm-frond
//! crown plus a couple upward spear fronds.

use bevy::prelude::*;
use sbs_geometry::render::mix_seed::node_mix_seed;
use sbs_geometry::{FrondCrownShape, FrondRachisSegment};
use vegetation_components::{FoliageNode, FrondCollection, FrondRun, Placement};

use crate::palm_tree::world_space_frond_shape;

/// VC-simplified palm crown (fewer leaflets / spine segments than RenderItem jungle growth).
const VC_FROND_COUNT: u32 = 5;
const VC_LEAFLET_COUNT: u32 = 5;
const VC_SPINE_SEGMENTS: u32 = 4;
/// Upward spears approximating Buddha's-hand fingers.
const SPEAR_COUNT: u32 = 2;

const FROND_CROWN_Y_FRACTION: f32 = 0.7;
const SPEAR_Y_FRACTION: f32 = 0.6;
/// Anchor offsets still track the legacy inner-ball radius fraction.
const INNER_BALL_SCALE: f32 = 0.72;
const BUDDHA_HAND_SCALE: f32 = 0.8;

fn mix_unit(node_idx: usize, position: Vec3, lane: u32) -> f32 {
	(node_mix_seed(node_idx, position).wrapping_add(lane) as f32) / (u32::MAX as f32)
}

fn jitter(center: f32, span: f32, t: f32) -> f32 {
	(center + (t - 0.5) * span).max(1e-4)
}

/// Authored growth cluster at a branch node (assembly uniform scale = `radius_scale`).
#[derive(Clone, Copy, Debug)]
pub(crate) struct JungleGrowthVcParams {
	pub node_idx: usize,
	pub position: Vec3,
	/// Spawn uniform scale driving frond size (Honu / Jungle Storybook radius centers).
	pub radius_scale: f32,
	pub foliage_scale: f32,
	pub seed: i32,
}

impl JungleGrowthVcParams {
	pub fn from_node(
		node_idx: usize,
		position: Vec3,
		radius_scale_center: f32,
		radius_scale_span: f32,
		foliage_scale_center: f32,
		foliage_scale_span: f32,
	) -> Self {
		let seed = (node_idx as i32)
			.wrapping_add(position.x.to_bits() as i32)
			.wrapping_add(position.y.to_bits().rotate_left(5) as i32);
		Self {
			node_idx,
			position,
			radius_scale: jitter(
				radius_scale_center,
				radius_scale_span,
				mix_unit(node_idx, position, 37),
			),
			foliage_scale: jitter(
				foliage_scale_center,
				foliage_scale_span,
				mix_unit(node_idx, position, 19),
			),
			seed,
		}
	}

	fn assembly_scale(self) -> f32 {
		self.radius_scale.max(1e-4)
	}

	fn foliage_world_scale(self) -> f32 {
		self.assembly_scale() * self.foliage_scale.max(1e-4)
	}
}

fn palm_frond_shape(params: JungleGrowthVcParams) -> FrondCrownShape {
	let local = FrondCrownShape {
		frond_count: VC_FROND_COUNT,
		length: 0.72,
		width: 0.11,
		droop: 0.42,
		arch_lift: 0.0,
		twist: 0.28,
		leaflet_count: VC_LEAFLET_COUNT,
		spine_segments: VC_SPINE_SEGMENTS,
		shoot_half_radius: 0.016,
		rachis_half_thickness: jitter(0.02, 0.004, mix_unit(params.node_idx, params.position, 11)),
		leaflet_length_scale: 1.7,
		downward_tilt_radians: 0.48,
		outward_spread_radians: 0.58,
		emission_lift_radians: 0.0,
		seed: params.seed.wrapping_add(31),
	};
	world_space_frond_shape(local, params.foliage_world_scale())
}

fn frond_runs_to_collection_runs(runs: Vec<Vec<FrondRachisSegment>>) -> Vec<FrondRun> {
	runs.into_iter()
		.filter_map(|run| {
			let placements: Vec<Placement> = run
				.into_iter()
				.filter_map(|seg| {
					Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
				})
				.collect();
			if placements.is_empty() {
				None
			} else {
				Some(FrondRun::from_placements(placements))
			}
		})
		.collect()
}

/// Two nearly-vertical spear fronds (Buddha-hand stand-in).
fn spear_frond_runs(params: JungleGrowthVcParams) -> Vec<FrondRun> {
	let origin =
		params.position + Vec3::Y * (params.assembly_scale() * INNER_BALL_SCALE * SPEAR_Y_FRACTION);
	let len = 0.55 * params.foliage_world_scale() * BUDDHA_HAND_SCALE;
	let width = 0.04 * params.foliage_world_scale() * BUDDHA_HAND_SCALE;
	let mut runs = Vec::with_capacity(SPEAR_COUNT as usize);
	for i in 0..SPEAR_COUNT {
		let yaw = (i as f32 + 0.35) * std::f32::consts::TAU / SPEAR_COUNT as f32
			+ (params.seed as f32) * 0.01;
		let tilt = 0.12 + 0.08 * mix_unit(params.node_idx, params.position, 41 + i);
		let dir = (Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-tilt) * Vec3::Y)
			.normalize_or_zero();
		if dir.length_squared() < 1e-8 {
			continue;
		}
		if let Some(placement) = Placement::frond_segment(origin, dir, len, width) {
			runs.push(FrondRun::from_placements(vec![placement]));
		}
	}
	runs
}

/// Frond collection only (palm crown + upward spears) — no growth inner ball.
pub(crate) fn jungle_growth_foliage_nodes(params: JungleGrowthVcParams) -> Vec<FoliageNode> {
	let frond_origin = params.position
		+ Vec3::Y * (params.assembly_scale() * INNER_BALL_SCALE * FROND_CROWN_Y_FRACTION);
	let mut runs =
		frond_runs_to_collection_runs(palm_frond_shape(params).frond_runs_at(frond_origin));
	runs.extend(spear_frond_runs(params));
	if runs.is_empty() {
		return Vec::new();
	}
	vec![FoliageNode::frond_collection(
		FrondCollection::new(runs).bake_bounds_from_runs(),
		Placement::IDENTITY,
	)]
}
