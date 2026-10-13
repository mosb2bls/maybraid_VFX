//! Joint [`FrondCrownShape`] sprays aligned to branch direction ([RFC §3.1.7.15](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/15-temperate-conifer/README.md), [#238](https://github.com/ramate-io/maybraid/issues/238)).

use bevy::prelude::*;
use procedural_common::UnitRange;
use sbs_geometry::render::mix_seed::node_mix_seed;
use sbs_geometry::{
	BallStickChain, BallStickNode, FriendsConiferChain, FriendsConiferSbs, FrondCrownShape,
};

const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.010;
const FROND_DROOP: f32 = 0.24;
const FROND_TWIST: f32 = 0.14;
/// VC-simplified fronds (fewer leaflets / spine segments → fewer collection members).
const FROND_LEAFLET_COUNT: u32 = 6;
const FROND_SPINE_SEGMENTS: u32 = 3;

/// Deterministic frond count in `fronds_per_joint` (inclusive range).
fn frond_count_for_node(node_idx: usize, position: Vec3, fronds_per_joint: &UnitRange) -> u32 {
	let span = fronds_per_joint.end - fronds_per_joint.start;
	if span <= 0.0 {
		return fronds_per_joint.start.round().max(1.0) as u32;
	}
	let t = (node_mix_seed(node_idx, position) as f32) / (u32::MAX as f32);
	let count = fronds_per_joint.start + t * span;
	count.round().clamp(1.0, 8.0) as u32
}

fn frond_length_world(
	h: f32,
	node_idx: usize,
	position: Vec3,
	length_fraction: &UnitRange,
	frond_world_scale: f32,
) -> f32 {
	let span = length_fraction.end - length_fraction.start;
	let t = (node_mix_seed(node_idx.wrapping_add(3), position) as f32) / (u32::MAX as f32);
	let frac = length_fraction.start + t * span;
	(frac * h / frond_world_scale.max(1e-8)).max(1e-4)
}

pub(crate) fn branch_direction(
	chain: &BallStickChain<FriendsConiferChain>,
	node_idx: usize,
	node: &BallStickNode,
) -> Vec3 {
	if let Some(parent) = chain.parent_index(node_idx) {
		let delta = node.position - chain.nodes[parent].position;
		if delta.length_squared() > 1e-10 {
			return delta.normalize();
		}
	}
	Vec3::Y
}

pub fn frond_shape_for_joint(
	geometry: &FriendsConiferSbs,
	frond_world_scale: f32,
	node_idx: usize,
	node: &BallStickNode,
	fronds_per_joint: &UnitRange,
	length_fraction: &UnitRange,
) -> FrondCrownShape {
	let h = geometry.height();
	let scale = frond_world_scale.max(1e-8);
	let seed = node_idx as i32 ^ node.position.x.to_bits() as i32;

	FrondCrownShape {
		frond_count: frond_count_for_node(node_idx, node.position, fronds_per_joint),
		length: frond_length_world(h, node_idx, node.position, length_fraction, scale),
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop: FROND_DROOP,
		arch_lift: 0.0,
		twist: FROND_TWIST,
		leaflet_count: FROND_LEAFLET_COUNT,
		spine_segments: FROND_SPINE_SEGMENTS,
		shoot_half_radius: 0.006,
		rachis_half_thickness: 0.003,
		leaflet_length_scale: 2.4,
		downward_tilt_radians: 0.12,
		outward_spread_radians: 0.08,
		emission_lift_radians: 0.0,
		seed,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sbs_geometry::FriendsConiferSbs;

	#[test]
	fn frond_shape_scales_with_height() {
		let mut sbs = FriendsConiferSbs::default();
		sbs.scale.stalk_height = 20.0;
		let node = BallStickNode::new(Vec3::new(0.0, 10.0, 0.0), 0.1);
		let length = UnitRange::new(0.035, 0.07);
		let count = UnitRange::new(1.0, 2.0);
		let shape = frond_shape_for_joint(&sbs, 1.0, 0, &node, &count, &length);
		assert!(shape.length > 0.5 && shape.length < 2.0);
		assert!(shape.width > 0.1);
	}
}
