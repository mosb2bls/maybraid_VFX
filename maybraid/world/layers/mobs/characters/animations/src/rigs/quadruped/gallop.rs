use std::f32::consts::PI;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{Gallop, QuadrupedGallop};
use crate::rigs::quadruped::apply::{apply_neck_axes, apply_spine};
use crate::rigs::quadruped::gait::{
	apply_front_leg_stride, apply_hind_leg_stride, KneeTuning, LegStrideTuning,
};
use crate::{Animation, Progress};

/// The full cycle spans two bounds; each leg strikes once per bound, so a leg's
/// phase covers two strikes per cycle.
const BOUNDS_PER_CYCLE: f32 = 2.0;

/// Delay between paired legs on the same girdle (cycle units). The lead leg of
/// a pair strikes first; the lead swaps every bound, which is what makes each
/// leg's two strides unequal in duration.
const PAIR_STAGGER: f32 = 0.06;

/// Where in each bound the spine is maximally gathered (just after the hind strikes).
const SPINE_GATHER_CENTER: f32 = 0.06;

impl Animation<QuadrupedV0Rig> for Gallop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		QuadrupedGallop::from_gallop(self).apply_for(rig, progress)
	}
}

impl Animation<QuadrupedV0Rig> for QuadrupedGallop {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let mut pose = QuadrupedPose::default();
		sample_gallop(self, progress, &mut pose);
		rig.write_pose(&pose);
	}
}

fn sample_gallop(gallop: &QuadrupedGallop, progress: f32, pose: &mut QuadrupedPose) {
	let cycle = Progress(progress).cycle();
	let bound_u = (cycle * BOUNDS_PER_CYCLE).fract();
	let tuning = leg_tuning(gallop);

	// Footfall order: BL, BR, FL, FR in the first bound, then BR, BL, FR, FL.
	for side in [Side::Left, Side::Right] {
		let (hind_first, hind_second) = strike_times(0.0, side);
		apply_hind_leg_stride(pose, side, gallop_leg_phase(cycle, hind_first, hind_second), tuning);

		let (front_first, front_second) = strike_times(gallop.phase_separation, side);
		apply_front_leg_stride(
			pose,
			side,
			gallop_leg_phase(cycle, front_first, front_second),
			tuning,
		);
	}

	let spine_flex = bound_spine_flex(bound_u, gallop.hind_bound_pitch, gallop.front_bound_pitch);
	apply_spine(pose, spine_flex * 0.35, spine_flex);
	apply_neck_axes(
		pose,
		-spine_flex * gallop.neck_follow,
		-spine_flex * gallop.neck_bow,
		-spine_flex * gallop.neck_pitch,
	);
}

/// Strike times over the full cycle for one leg of a pair whose lead leg
/// strikes at `pair_strike` in the first bound.
///
/// The left leg leads the first bound and trails the second; the right leg
/// mirrors it. The two strikes are therefore `0.5 + PAIR_STAGGER` apart for one
/// leg and `0.5 - PAIR_STAGGER` apart for its partner.
fn strike_times(pair_strike: f32, side: Side) -> (f32, f32) {
	match side {
		Side::Left => (pair_strike, pair_strike + 0.5 + PAIR_STAGGER),
		Side::Right => (pair_strike + PAIR_STAGGER, pair_strike + 0.5),
	}
}

/// Continuous leg phase in `[0, 1)` with `0` at each strike.
///
/// The phase advances linearly between consecutive strikes, so the stride
/// spanning the longer inter-strike interval plays slightly slower and the
/// other slightly faster — the uneven two-beat rhythm of the gallop.
fn gallop_leg_phase(cycle: f32, first_strike: f32, second_strike: f32) -> f32 {
	let first_interval = second_strike - first_strike;
	let second_interval = 1.0 - first_interval;
	let since_first = (cycle - first_strike).rem_euclid(1.0);
	if since_first < first_interval {
		since_first / first_interval
	} else {
		(since_first - first_interval) / second_interval
	}
}

/// Spine gathers (positive flex) around the hind strikes at the start of each
/// bound and extends (negative flex) around the front strikes mid-bound.
fn bound_spine_flex(bound_u: f32, hind_pitch: f32, front_pitch: f32) -> f32 {
	let wave = ((bound_u - SPINE_GATHER_CENTER) * 2.0 * PI).cos();
	if wave >= 0.0 {
		wave * hind_pitch
	} else {
		wave * front_pitch
	}
}

fn leg_tuning(gallop: &QuadrupedGallop) -> LegStrideTuning {
	LegStrideTuning {
		shoulder_swing: gallop.shoulder_swing,
		shoulder_lift: gallop.shoulder_lift,
		hip_swing: gallop.hip_swing,
		hip_lift: gallop.hip_lift,
		stride: gallop.stride,
		knee: KneeTuning {
			knee_neutral: gallop.knee_neutral,
			knee_contracted: gallop.knee_contracted,
			knee_extended: gallop.knee_extended,
		},
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	fn tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn gallop_delegates_to_quadruped_default() {
		for &phase in &[0.0, 0.12, 0.25, 0.62, 0.75] {
			let mut from_gallop = QuadrupedV0Rig::imported();
			let mut from_template = QuadrupedV0Rig::imported();
			Gallop::default().apply(&mut from_gallop, phase);
			QuadrupedGallop::default().apply(&mut from_template, phase);

			for name in from_gallop.animation_bone_names() {
				let gallop_rot = from_gallop.rotation(name);
				let template_rot = from_template.rotation(name);
				assert!(
					gallop_rot.dot(template_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn gallop_footfall_order_swaps_lead_between_bounds() {
		let phase_separation = QuadrupedGallop::default().phase_separation;
		let (hind_left_first, hind_left_second) = strike_times(0.0, Side::Left);
		let (hind_right_first, hind_right_second) = strike_times(0.0, Side::Right);
		let (front_left_first, front_left_second) = strike_times(phase_separation, Side::Left);
		let (front_right_first, front_right_second) = strike_times(phase_separation, Side::Right);

		// First bound: BL, BR, FL, FR.
		assert!(hind_left_first < hind_right_first);
		assert!(hind_right_first < front_left_first);
		assert!(front_left_first < front_right_first);

		// Second bound: BR, BL, FR, FL.
		assert!(hind_right_second < hind_left_second);
		assert!(hind_left_second < front_right_second);
		assert!(front_right_second < front_left_second);
	}

	#[test]
	fn gallop_phase_separation_offsets_front_strikes() {
		let mut tight = QuadrupedGallop::default();
		let mut wide = QuadrupedGallop::default();
		tight.phase_separation = 0.12;
		wide.phase_separation = 0.32;

		let (tight_first, _) = strike_times(tight.phase_separation, Side::Left);
		let (wide_first, _) = strike_times(wide.phase_separation, Side::Left);
		assert!(wide_first > tight_first);
	}

	#[test]
	fn gallop_leg_strides_alternate_fast_and_slow() {
		let (first, second) = strike_times(0.0, Side::Left);
		let first_interval = second - first;
		let second_interval = 1.0 - first_interval;
		assert!(
			first_interval > second_interval,
			"left hind's first stride should span the longer interval"
		);

		// Halfway into each interval the phase reads 0.5: the shorter stride
		// covers the same phase distance in less cycle time.
		let mid_first = gallop_leg_phase(first + first_interval * 0.5, first, second);
		let mid_second = gallop_leg_phase(second + second_interval * 0.5, first, second);
		assert!((mid_first - 0.5).abs() < 1e-5);
		assert!((mid_second - 0.5).abs() < 1e-5);

		// Equal cycle time after each strike yields more phase progress on the
		// faster (shorter) stride.
		let dt = 0.1;
		let after_first = gallop_leg_phase(first + dt, first, second);
		let after_second = gallop_leg_phase(second + dt, first, second);
		assert!(after_second > after_first, "second stride should run faster for the left hind");
	}

	#[test]
	fn gallop_hind_pair_stays_near_phase_during_hind_bound() {
		let mut a = QuadrupedV0Rig::imported();
		let mut b = QuadrupedV0Rig::imported();
		QuadrupedGallop::default().apply(&mut a, 0.03);
		QuadrupedGallop::default().apply(&mut b, 0.28);
		let bound = a.rotation("posterior_thigh.L").angle_between(a.rotation("posterior_thigh.R"));
		let split = b.rotation("posterior_thigh.L").angle_between(b.rotation("posterior_thigh.R"));
		assert!(
			bound < split,
			"hind pair should be closer during the bound than mid-cycle, bound={bound} split={split}"
		);
	}

	#[test]
	fn gallop_legs_are_continuous_between_adjacent_samples() {
		let gallop = QuadrupedGallop::default();
		let bones =
			["posterior_thigh.L", "posterior_thigh.R", "anterior_thigh.L", "anterior_thigh.R"];

		let sample = |phase: f32| {
			let mut rig = QuadrupedV0Rig::imported();
			gallop.apply(&mut rig, phase);
			bones.map(|name| rig.rotation(name))
		};

		let mut prev = sample(0.0);
		for step in 1..=200 {
			let phase = step as f32 / 200.0;
			let rots = sample(phase);
			for (bone, (rot, prev_rot)) in bones.iter().zip(rots.iter().zip(&prev)) {
				let delta = rot.angle_between(*prev_rot);
				assert!(delta < 0.08, "leg jerk on {bone} at phase {phase}: delta={delta}");
			}
			prev = rots;
		}
	}

	#[test]
	fn gallop_spine_is_continuous_between_adjacent_samples() {
		let gallop = QuadrupedGallop::default();
		let mut rig_prev = QuadrupedV0Rig::imported();
		gallop.apply(&mut rig_prev, 0.0);
		let mut prev = rig_prev.rotation("lumbar");

		for step in 1..=200 {
			let phase = step as f32 / 200.0;
			let mut rig = QuadrupedV0Rig::imported();
			gallop.apply(&mut rig, phase);
			let rot = rig.rotation("lumbar");
			let delta = rot.angle_between(prev);
			assert!(delta < 0.08, "spine jerk at phase {phase}: delta={delta}");
			prev = rot;
		}
	}

	#[test]
	fn gallop_neck_bows_and_rotates_with_the_bound() {
		let mut gathered = QuadrupedV0Rig::imported();
		let mut extended = QuadrupedV0Rig::imported();
		QuadrupedGallop::default().apply(&mut gathered, 0.03);
		QuadrupedGallop::default().apply(&mut extended, 0.28);

		let gather_y = tip(&gathered, "neck");
		let extend_y = tip(&extended, "neck");
		let gather_yaw = gathered.rotation("neck") * Vec3::Z;
		assert!(gather_y.x > 0.05, "gathered tilt is lateral, got {gather_y:?}");
		assert!(gather_yaw.x.abs() > 0.03, "gathered turn is yaw, got {gather_yaw:?}");
		assert!(gather_y.z < -0.04, "gathered nod is down, got {gather_y:?}");
		assert!(extend_y.z > gather_y.z, "extended bound should lift the nod");
	}

	#[test]
	fn gallop_spine_gathers_at_hind_strike_and_extends_at_front_strike() {
		let gallop = QuadrupedGallop::default();
		let gathered = bound_spine_flex(
			SPINE_GATHER_CENTER,
			gallop.hind_bound_pitch,
			gallop.front_bound_pitch,
		);
		let extended = bound_spine_flex(
			SPINE_GATHER_CENTER + 0.5,
			gallop.hind_bound_pitch,
			gallop.front_bound_pitch,
		);
		assert!(gathered > 0.0);
		assert!(extended < 0.0);
	}
}
