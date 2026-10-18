//! Bone write masks for cyclic locomotion clips.
//!
//! Bone name lists are the single source of truth for both live `apply_for` and
//! [`character_motion::clip_cache`] prepared tables. Tests union
//! [`humanoid_pose_write_mask`] across sampled progress values and fail when a
//! clip writes outside its mask.

use std::sync::OnceLock;

use character_rigs::authoring::{humanoid_pose_write_mask, humanoid_write_mask, HumanoidPose};

pub const IDLE_WRITE_BONES: &[&str] = &[
	"lower_neck",
	"upper_neck",
	"pelvis.L",
	"pelvis.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

pub const WALK_WRITE_BONES: &[&str] = &[
	"root",
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

pub const RUN_WRITE_BONES: &[&str] = &[
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

pub fn idle_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(IDLE_WRITE_BONES))
}

pub fn walk_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(WALK_WRITE_BONES))
}

pub fn run_write_mask() -> u32 {
	static MASK: OnceLock<u32> = OnceLock::new();
	*MASK.get_or_init(|| humanoid_write_mask(RUN_WRITE_BONES))
}

/// Assert in debug builds that `pose` does not write outside `mask`.
#[inline]
pub fn debug_assert_pose_within_mask(pose: &HumanoidPose, mask: u32, clip: &str) {
	let written = humanoid_pose_write_mask(pose);
	debug_assert!(
		written & !mask == 0,
		"{clip} wrote bones outside its mask: written={written:#034b} mask={mask:#034b}"
	);
}

#[cfg(test)]
mod tests {
	use character_rigs::authoring::resolve_humanoid;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use crate::animations::{Idle, Run, Walk};
	use crate::Animation;

	use super::*;

	fn sample_phases() -> Vec<f32> {
		(0..=40).map(|i| i as f32 / 40.0).collect()
	}

	fn union_pose_mask<F>(sample: F) -> u32
	where
		F: Fn(f32) -> HumanoidPose,
	{
		sample_phases()
			.iter()
			.map(|&progress| humanoid_pose_write_mask(&sample(progress)))
			.fold(0, |mask, bits| mask | bits)
	}

	#[test]
	fn static_masks_cover_all_authored_bones() {
		let cases = [
			("idle", idle_write_mask(), union_pose_mask(|p| Idle::default().sample_pose(p))),
			("walk", walk_write_mask(), union_pose_mask(|p| Walk::default().sample_pose(p))),
			("run", run_write_mask(), union_pose_mask(|p| Run::default().sample_pose(p))),
		];
		for (clip, mask, authored) in cases {
			assert_eq!(
				mask, authored,
				"{clip} mask must match union of sampled pose masks (mask={mask:#034b} authored={authored:#034b})"
			);
		}
	}

	#[test]
	fn masked_apply_matches_full_resolve_after_foreign_clip() {
		let phases = sample_phases();
		let cases: [(&str, u32, fn(f32) -> HumanoidPose); 3] = [
			("idle", idle_write_mask(), |p| Idle::default().sample_pose(p)),
			("walk", walk_write_mask(), |p| Walk::default().sample_pose(p)),
			("run", run_write_mask(), |p| Run::default().sample_pose(p)),
		];
		let seed_clips: [(&str, fn(&mut HumanoidV0Rig, f32)); 2] = [
			("walk", |rig, p| Walk::default().apply_for(rig, p)),
			("run", |rig, p| Run::default().apply_for(rig, p)),
		];

		for (clip, mask, sample) in cases {
			for &phase in &phases {
				let pose = sample(phase);
				debug_assert_pose_within_mask(&pose, mask, clip);

				let mut expected = HumanoidV0Rig::imported();
				resolve_humanoid(&pose, &expected.binding, &mut expected.pose);

				for (seed_name, seed) in &seed_clips {
					let mut actual = HumanoidV0Rig::imported();
					seed(&mut actual, phase * 0.37 + 0.11);
					actual.apply_masked_pose(&pose, mask);
					assert_eq!(
						expected.pose.local, actual.pose.local,
						"{clip} at {phase} must not retain {seed_name} bones after masked apply"
					);
				}
			}
		}
	}
}
