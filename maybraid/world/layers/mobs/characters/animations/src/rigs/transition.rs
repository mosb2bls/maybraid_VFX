use character_rigs::authoring::PoseBuffer;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Transition;
use crate::rigs::mix::{capture_into, mix_effects};
use crate::{Animation, Effects};

impl<A> Transition<A>
where
	A: Animation<HumanoidV0Rig>,
{
	/// Creates a transition into `animation`, capturing the rig's current pose and
	/// treating the current armature offset as identity unless the caller stored one.
	pub fn new(animation: A, rig: &HumanoidV0Rig) -> Self {
		Self::from_pose(animation, rig.pose.clone())
	}

	pub fn apply(
		&self,
		rig: &mut HumanoidV0Rig,
		animation_progress: f32,
		transition_progress: f32,
	) -> Effects {
		let weight = self.weight(transition_progress);
		if weight <= 0.0 {
			rig.pose.copy_from(&self.from_pose);
			return self.from_offset;
		}
		self.animation.apply_for(rig, animation_progress);
		let effects = self.animation.effects_for(rig, animation_progress);
		if weight >= 1.0 {
			return mix_effects(self.from_offset, effects, 1.0);
		}
		rig.scratch.b.copy_from(&rig.pose);
		PoseBuffer::blend_into(&self.from_pose, &rig.scratch.b, weight, &mut rig.pose);
		mix_effects(self.from_offset, effects, weight)
	}
}

/// Samples an animation into an owned buffer. Prefer [`capture_into`] on a reused slot.
pub fn capture_animation_pose<A>(anim: &A, rig: &mut HumanoidV0Rig, progress: f32) -> PoseBuffer
where
	A: Animation<HumanoidV0Rig>,
{
	let mut dest = PoseBuffer::identity(rig.pose.len());
	capture_into(anim, rig, progress, &mut dest);
	dest
}

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use super::*;
	use bevy::prelude::*;

	use crate::animations::{Fall, Land, Spring, Squat, Transition, TransitionCurve};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	fn apply_legacy<A>(
		transition: &Transition<A>,
		rig: &mut HumanoidV0Rig,
		animation_progress: f32,
		transition_progress: f32,
	) -> Effects
	where
		A: Animation<HumanoidV0Rig>,
	{
		transition.animation.apply_for(rig, animation_progress);
		let effects = transition.animation.effects_for(rig, animation_progress);
		rig.scratch.b.copy_from(&rig.pose);
		let weight = transition.weight(transition_progress);
		PoseBuffer::blend_into(&transition.from_pose, &rig.scratch.b, weight, &mut rig.pose);
		mix_effects(transition.from_offset, effects, weight)
	}

	#[test]
	fn endpoint_apply_matches_legacy_pose_and_effects() -> anyhow::Result<()> {
		let mut source = HumanoidV0Rig::imported();
		let from_pose = capture_animation_pose(&Fall::default(), &mut source, 0.75);
		let from_offset = ArmatureOffset::from_translation(Vec3::new(0.0, -0.1, 0.0));
		let transition = Transition::from_visible(Spring::default(), from_pose, from_offset)
			.with_curve(TransitionCurve::EaseInOut);

		for transition_progress in [0.0, 1.0] {
			for animation_progress in [0.0, 0.35, 0.9] {
				let mut legacy = HumanoidV0Rig::imported();
				let legacy_effects =
					apply_legacy(&transition, &mut legacy, animation_progress, transition_progress);
				let mut optimized = HumanoidV0Rig::imported();
				let optimized_effects =
					transition.apply(&mut optimized, animation_progress, transition_progress);
				for name in legacy.animation_bone_names() {
					assert!(
						legacy.rotation(name).dot(optimized.rotation(name)).abs() > 1.0 - 1e-5,
						"pose mismatch at anim={animation_progress} trans={transition_progress} on {name}"
					);
				}
				assert_eq!(legacy_effects, optimized_effects);
			}
		}
		Ok(())
	}

	fn bench_transition_apply(legacy: bool, transition_progress: f32) -> Vec<u128> {
		const FRAMES: u32 = 5_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let mut source = HumanoidV0Rig::imported();
		Fall::default().apply(&mut source, 1.0);
		let from_pose = source.pose.clone();
		let transition = Transition::from_pose(Squat::for_loop(1.0, 1.0), from_pose);

		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let animation_progress =
						black_box((frame as f32 * 0.011 + index as f32 * 0.03).rem_euclid(1.0));
					if legacy {
						black_box(apply_legacy(
							&transition,
							rig,
							animation_progress,
							transition_progress,
						));
					} else {
						black_box(transition.apply(
							rig,
							animation_progress,
							black_box(transition_progress),
						));
					}
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	/// `cargo test -p character-animations transition_apply_endpoints_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn transition_apply_endpoints_microbench() {
		eprintln!(
			"32 characters × 5000 frames, Transition Spring→Squat, real Transition::apply path"
		);
		for (label, progress) in [("weight 0.0", 0.0), ("weight 1.0", 1.0), ("weight 0.5", 0.5)] {
			let legacy = bench_transition_apply(true, progress);
			let optimized = bench_transition_apply(false, progress);
			eprintln!(
				"transition_apply_endpoints_microbench {label} legacy: min={} median={} ns/sample",
				legacy.first().expect("run"),
				legacy[legacy.len() / 2]
			);
			eprintln!(
				"transition_apply_endpoints_microbench {label} optimized: min={} median={} ns/sample",
				optimized.first().expect("run"),
				optimized[optimized.len() / 2]
			);
		}
	}

	#[test]
	fn transition_at_zero_matches_from_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Fall::default().apply(&mut rig, 1.0);
		let from_pose = rig.pose.clone();

		let mut out = HumanoidV0Rig::imported();
		Transition::from_pose(Squat::for_loop(1.0, 1.0), from_pose).apply(&mut out, 0.5, 0.0);

		assert!(rig.rotation("femur.L").dot(out.rotation("femur.L")).abs() > 1.0 - 1e-5);
		Ok(())
	}

	#[test]
	fn transition_at_one_matches_target_animation() -> anyhow::Result<()> {
		let rig = HumanoidV0Rig::imported();
		let from_pose = rig.pose.clone();
		let target = Squat::for_loop(1.0, 1.0);

		let mut expected = HumanoidV0Rig::imported();
		target.apply(&mut expected, 0.5);

		let mut out = HumanoidV0Rig::imported();
		Transition::from_pose(target, from_pose).apply(&mut out, 0.5, 1.0);

		assert!(expected.rotation("femur.L").dot(out.rotation("femur.L")).abs() > 1.0 - 1e-5);
		Ok(())
	}

	#[test]
	fn fall_to_land_transition_blends_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = capture_animation_pose(&Fall::default(), &mut rig, 1.0);
		let land = Land::default();
		let land_progress = 0.05 / land.cycle_duration();

		Transition::from_pose(land, from_pose)
			.with_curve(TransitionCurve::SmoothStep)
			.apply(&mut rig, land_progress, 0.5);

		assert!(rig.posed_angle("shoulder.L") > 0.02, "blended shoulder leaves rest");
		Ok(())
	}

	#[test]
	fn spring_transition_from_stand() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = capture_animation_pose(&Squat::for_loop(1.0, 1.0), &mut rig, 0.0);

		Transition::from_pose(Spring::default(), from_pose).apply(&mut rig, 0.5, 0.5);

		assert!(rig.posed_angle("femur.L") > 0.0);
		let mut full = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut full, 0.5);
		assert!(rig.posed_angle("femur.L") < full.posed_angle("femur.L"));
		Ok(())
	}

	#[test]
	fn interrupted_transition_starts_from_the_visible_offset() {
		let mut rig = HumanoidV0Rig::imported();
		let from_pose = rig.pose.clone();
		let from_offset = ArmatureOffset::from_translation(Vec3::new(0.0, -0.2, 0.0));
		let effects = Transition::from_visible(Squat::for_loop(1.0, 1.0), from_pose, from_offset)
			.apply(&mut rig, 0.0, 0.0);
		assert!((effects.0.translation - from_offset.0.translation).length() < 1e-5);
	}
}
