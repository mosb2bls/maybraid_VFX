//! Stochastic apex canopy at the stalk crown (Friend's ball, Temperate fronds).

use procedural_common::NoiseParams;
use sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use sbs_geometry::BallStickNode;

/// Default fraction of trees that receive an apex canopy cluster (deterministic from [`NoiseParams`] + tip).
pub const DEFAULT_APEX_CANOPY_SPAWN_FRACTION: f32 = 0.72;

/// Friend's apex ball world radius as a fraction of stalk height.
pub const FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.024;

/// Northern Conifer apex ball world radius as a fraction of stalk height.
pub const NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.022;

/// Gate apex foliage from leaf noise and stalk-tip position.
pub fn sample_apex_canopy_spawn(
	leaf_noise: &NoiseParams,
	tip: &BallStickNode,
	spawn_fraction: f32,
) -> bool {
	let lane = leaf_noise.seed.wrapping_mul(0x9E37) as usize;
	mix_seed_below_fraction(lane, tip.position, spawn_fraction)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use sbs_geometry::{liams_stalk_tip_from_chain, FriendsConiferSbs};

	#[test]
	fn apex_gate_is_deterministic() {
		let noise = NoiseParams::from_scalar(42.0, 1.0, 1.0, 1);
		let tip = BallStickNode::new(Vec3::new(0.0, 30.0, 0.0), 0.2);
		let a = sample_apex_canopy_spawn(&noise, &tip, 0.72);
		let b = sample_apex_canopy_spawn(&noise, &tip, 0.72);
		assert_eq!(a, b);
	}

	#[test]
	fn apex_gate_respects_fraction_bounds() {
		let noise = NoiseParams::from_scalar(7.0, 1.0, 1.0, 1);
		let tip = BallStickNode::new(Vec3::new(1.0, 25.0, 2.0), 0.15);
		assert!(!sample_apex_canopy_spawn(&noise, &tip, 0.0));
		assert!(sample_apex_canopy_spawn(&noise, &tip, 1.0));
	}

	#[test]
	fn stalk_tip_from_built_friends_chain() {
		let chain = FriendsConiferSbs::default().build_chain();
		let tip = liams_stalk_tip_from_chain(&chain);
		assert!(tip.position.y > 20.0);
	}
}
