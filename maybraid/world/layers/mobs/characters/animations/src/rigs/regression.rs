//! Clip samples on inspected GLB rest. These checks are directional: a twist
//! that only leaves rest must not pass.

use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::{Flapping, Jab, QuadrupedRun, Squat, ThinkAgain, Walk};
use crate::Animation;

#[test]
fn squat_folds_the_t_pose_thigh_in_z() {
	let rest = HumanoidV0Rig::for_clip_test();
	let mut posed = HumanoidV0Rig::for_clip_test();
	Squat::held().apply(&mut posed, 1.0);
	let rest_dir = rest.character_length("femur.L");
	let posed_dir = posed.character_length("femur.L");
	assert!(posed_dir.z.abs() > rest_dir.z.abs() + 0.2, "squat is sagittal, {posed_dir:?}");
	assert!(posed_dir.x.abs() < 0.08, "squat is not a side swing, {posed_dir:?}");
	let rest_knee = rest.character_point("shin.L");
	let posed_knee = posed.character_point("shin.L");
	assert!(
		(posed_knee.z - rest_knee.z).abs() > 0.05,
		"knee must move front/back, {posed_knee:?} vs {rest_knee:?}"
	);
}

#[test]
fn walk_strides_the_seeded_femur_front_to_back() {
	let mut a = HumanoidV0Rig::for_clip_test();
	let mut b = HumanoidV0Rig::for_clip_test();
	Walk::default().apply(&mut a, 0.0);
	Walk::default().apply(&mut b, 0.5);
	let left_a = a.character_length("femur.L");
	let left_b = b.character_length("femur.L");
	assert!((left_a.z - left_b.z).abs() > 0.15, "stride is Z, {left_a:?} vs {left_b:?}");
	assert!((left_a.x - left_b.x).abs() < 0.12, "stride is not sideways, {left_a:?} vs {left_b:?}");
}

#[test]
fn flapping_keeps_the_previous_front_back_stroke() {
	let flap = Flapping { speed: 1.0, range: 1.0 };
	let mut front = HumanoidV0Rig::for_clip_test();
	let mut back = HumanoidV0Rig::for_clip_test();
	flap.apply(&mut front, 0.25);
	flap.apply(&mut back, 0.75);
	let a = front.character_length("forearm.L");
	let b = back.character_length("forearm.L");
	assert!((a.z - b.z).abs() > (a.y - b.y).abs(), "previous flap is XZ, {a:?} vs {b:?}");
	assert!((a - b).length() > 0.3, "wing tip must move, {a:?} vs {b:?}");
}

#[test]
fn jab_cover_elbow_tucks_in_y_not_as_a_roll() {
	let mut rig = HumanoidV0Rig::for_clip_test();
	Jab::default().apply(&mut rig, 0.47);
	let rest = HumanoidV0Rig::for_clip_test();
	let cover = rig.character_length("forearm.L");
	let rest_cover = rest.character_length("forearm.L");
	assert!(cover.y.abs() > 0.15, "cover elbow changes the hinge, {cover:?}");
	assert!(
		(cover - rest_cover).length() > 0.2,
		"cover is not a length-axis roll, {cover:?} vs {rest_cover:?}"
	);
}

#[test]
fn think_again_sweeps_forearm_beside_the_head() {
	let rest = HumanoidV0Rig::for_clip_test();
	let mut think = HumanoidV0Rig::for_clip_test();
	let mut again = HumanoidV0Rig::for_clip_test();
	ThinkAgain::default().apply(&mut think, 1.50);
	ThinkAgain::default().apply(&mut again, 3.40);
	let rest_dir = rest.character_length("forearm.R");
	let think_dir = think.character_length("forearm.R");
	let again_dir = again.character_length("forearm.R");
	assert!(think_dir.y > rest_dir.y + 0.3, "Think forearm rises, {think_dir:?}");
	assert!(again_dir.y > rest_dir.y + 0.3, "Again forearm stays up, {again_dir:?}");
	assert!(
		again_dir.y > think_dir.y,
		"Again is closer to vertical than Think, think {think_dir:?} again {again_dir:?}"
	);
}

#[test]
fn quadruped_run_folds_the_front_shin_backward() {
	let s2 = 0.70710677;
	let mid = bevy::prelude::Quat::from_xyzw(0.0, s2, s2, 0.0);
	let rest = QuadrupedV0Rig::for_clip_test();
	let rest_visual = mid
		* rest.rotation("shoulder.L")
		* rest.rotation("anterior_thigh.L")
		* rest.rotation("anterior_shin.L")
		* bevy::prelude::Vec3::Y;
	let mut best = rest_visual;
	let mut best_angle = 0.0;
	for phase in [0.0, 0.1, 0.2, 0.35, 0.5, 0.65, 0.8] {
		let mut posed = QuadrupedV0Rig::for_clip_test();
		QuadrupedRun::default().apply(&mut posed, phase);
		let angle = posed.posed_angle("anterior_shin.L");
		if angle >= best_angle {
			best_angle = angle;
			best = mid
				* posed.rotation("shoulder.L")
				* posed.rotation("anterior_thigh.L")
				* posed.rotation("anterior_shin.L")
				* bevy::prelude::Vec3::Y;
		}
	}
	assert!(best_angle > 0.2, "run must flex a front shin");
	assert!(best.z < rest_visual.z - 0.05, "hinge folds back, {best:?} vs {rest_visual:?}");
	assert!(best.x.abs() < 0.2, "hinge stays sagittal, {best:?}");
}
