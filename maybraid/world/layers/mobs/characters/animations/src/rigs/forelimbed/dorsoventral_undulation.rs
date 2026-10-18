use character_rigs::authoring::ForelimbedPose;
use character_rigs::rigs::forelimbed_v0::ForelimbedV0Rig;
use character_rigs::Side;

use crate::animations::DorsoventralUndulation;
use crate::Animation;

const SEGMENT_COUNT: usize = 4;
const PITCH_SCALE: f32 = 0.4;
const FIN_FLEX: f32 = 0.1;

impl Animation<ForelimbedV0Rig> for DorsoventralUndulation {
	fn apply_for(&self, rig: &mut ForelimbedV0Rig, progress: f32) {
		let mut pose = ForelimbedPose::default();
		let pitches = [
			self.segment_pitch(progress, 0, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 1, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 2, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 3, SEGMENT_COUNT) * PITCH_SCALE,
		];

		// Spine order: upper_mid, upper, lower_mid, lower, tailbone.
		// Dorsoventral bend is flexion about +X. Upper shares the first sample.
		pose.dorsoventral =
			[pitches[0] * 0.35, pitches[0] * 0.55, pitches[1], pitches[2], pitches[3]];

		let fin_phase = self.wave_phase(progress);
		for side in [Side::Left, Side::Right] {
			let paddle = (std::f32::consts::TAU * fin_phase).sin() * FIN_FLEX;
			// Upper arm flap is scaled by 0.6 inside the resolver.
			pose.fin_flap[side.index()] = paddle;
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	#[test]
	fn dorsoventral_bend_is_sagittal_and_flap_is_lateral() {
		let swim = DorsoventralUndulation::default();
		let mut rig = ForelimbedV0Rig::imported();
		swim.apply(&mut rig, 0.25 / swim.speed);

		let bend = rig.rotation("upper_spine") * Vec3::Y;
		assert!(bend.z.abs() > 0.05, "dorsoventral bend, got {bend:?}");
		assert!(bend.x.abs() < 1e-3, "bend stays sagittal, got {bend:?}");

		let shoulder = rig.rotation("shoulder.L") * Vec3::Y;
		let upper = rig.rotation("upper_arm.L") * Vec3::Y;
		assert!(shoulder.x.abs() > 0.02, "flap is lateral, got {shoulder:?}");
		assert!(
			shoulder.x.abs() > upper.x.abs(),
			"resolver scales the upper arm, shoulder {shoulder:?} upper {upper:?}"
		);
	}
}
