//! Animated bounds envelopes for Bray and sleep clips (rig sampling).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Vec3;
use character_animations::animations::{
	EpiphantBray, EpiphantSleepEnter, EpiphantSleepHold, EpiphantWake,
};
use character_animations::Animation;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::bounds::visual_bounds_aabb;
use crate::recipe::BikanwardaActor;

/// Regression sample for animated bounds (label, normalized progress).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationBoundsSample {
	pub label: &'static str,
	pub progress: f32,
}

pub const BIKANWARDA_ANIMATION_SAMPLES: &[AnimationBoundsSample] = &[
	AnimationBoundsSample { label: "bray-apex", progress: 0.34 },
	AnimationBoundsSample { label: "sleep-enter-settle", progress: 0.90 },
	AnimationBoundsSample { label: "sleep-hold-breathe", progress: 0.35 },
	AnimationBoundsSample { label: "wake-front-plant", progress: 0.18 },
];

const NECK_ANGLE_TO_HEIGHT: f32 = 0.55;

/// Union of rest visual bounds and neck-motion margin for authored clips.
pub fn animated_bounds_envelope(actor: &BikanwardaActor) -> Aabb3d {
	let rest = visual_bounds_aabb(actor);
	let scale = actor.uniform_scale;
	let mut min = Vec3::from(rest.min);
	let mut max = Vec3::from(rest.max);

	let mut rig = QuadrupedV0Rig::for_clip_test();
	let rest_neck = rig.posed_angle("neck");
	let mut peak = 0.0_f32;
	for sample in BIKANWARDA_ANIMATION_SAMPLES {
		apply_sample(&mut rig, *sample);
		peak = peak.max((rig.posed_angle("neck") - rest_neck).abs());
	}
	let expand_y = peak * NECK_ANGLE_TO_HEIGHT * scale;
	let lateral = peak * 0.25 * scale;
	min.y -= expand_y * 0.35;
	max.y += expand_y;
	min.x -= lateral;
	max.x += lateral;
	min.z -= lateral;
	max.z += lateral;
	Aabb3d::from_min_max(min, max)
}

fn apply_sample(rig: &mut QuadrupedV0Rig, sample: AnimationBoundsSample) {
	match sample.label {
		"bray-apex" => EpiphantBray::default().apply_for(rig, sample.progress),
		"sleep-enter-settle" => EpiphantSleepEnter::default().apply_for(rig, sample.progress),
		"sleep-hold-breathe" => EpiphantSleepHold::default().apply_for(rig, sample.progress),
		"wake-front-plant" => EpiphantWake::default().apply_for(rig, sample.progress),
		_ => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::recipe::BikanwardaActorRecipe;
	use characters::CharacterRecipe;

	#[test]
	fn animated_envelope_covers_rest_bounds() {
		let actor = BikanwardaActorRecipe::default().components();
		let rest = visual_bounds_aabb(&actor);
		let animated = animated_bounds_envelope(&actor);
		let rmin = Vec3::from(rest.min);
		let rmax = Vec3::from(rest.max);
		let amin = Vec3::from(animated.min);
		let amax = Vec3::from(animated.max);
		assert!(amin.x <= rmin.x + 1e-3 && amin.y <= rmin.y + 1e-3);
		assert!(amax.x >= rmax.x - 1e-3 && amax.y >= rmax.y - 1e-3);
	}

	#[test]
	fn bray_apex_expands_bounds_over_still() {
		let actor = BikanwardaActorRecipe::default().components();
		let rest_h = Vec3::from(visual_bounds_aabb(&actor).max).y
			- Vec3::from(visual_bounds_aabb(&actor).min).y;
		let anim_h = Vec3::from(animated_bounds_envelope(&actor).max).y
			- Vec3::from(animated_bounds_envelope(&actor).min).y;
		assert!(anim_h >= rest_h);
	}
}
