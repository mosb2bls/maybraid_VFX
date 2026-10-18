use character_rigs::authoring::ForelimbedPose;
use character_rigs::rigs::forelimbed_v0::ForelimbedV0Rig;
use character_rigs::Side;

use crate::animations::LateralUndulation;
use crate::Animation;

const SEGMENT_COUNT: usize = 4;
const YAW_SCALE: f32 = 0.45;
const FIN_SWING: f32 = 0.12;

impl Animation<ForelimbedV0Rig> for LateralUndulation {
	fn apply_for(&self, rig: &mut ForelimbedV0Rig, progress: f32) {
		let mut pose = ForelimbedPose::default();
		let yaws = [
			self.segment_yaw(progress, 0, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 1, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 2, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 3, SEGMENT_COUNT) * YAW_SCALE,
		];

		// Spine order: upper_mid, upper, lower_mid, lower, tailbone.
		// Upper shares the first sample; the resolver turns lateral into axial +Y.
		pose.lateral = [yaws[0] * 0.35, yaws[0] * 0.55, yaws[1], yaws[2], yaws[3]];

		let fin_phase = self.wave_phase(progress);
		for side in [Side::Left, Side::Right] {
			let lateral = match side {
				Side::Left => 1.0,
				Side::Right => -1.0,
			};
			let paddle = (std::f32::consts::TAU * fin_phase).sin() * FIN_SWING * lateral;
			// Upper arm is scaled by 0.6 inside the resolver.
			pose.fin_sweep[side.index()] = paddle;
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	#[test]
	fn lateral_yaw_is_axial_and_fin_sweep_is_unscaled() {
		let swim = LateralUndulation::default();
		let mut rig = ForelimbedV0Rig::imported();
		swim.apply(&mut rig, 0.25 / swim.speed);

		let yaw = rig.rotation("upper_spine") * Vec3::Z;
		assert!(yaw.x.abs() > 0.05, "lateral undulation yaws, got {yaw:?}");
		assert!(yaw.y.abs() < 0.05, "yaw stays level, got {yaw:?}");

		let shoulder = rig.rotation("shoulder.L") * Vec3::Z;
		let upper = rig.rotation("upper_arm.L") * Vec3::Z;
		assert!(
			shoulder.x.abs() > upper.x.abs(),
			"resolver scales the upper arm, shoulder {shoulder:?} upper {upper:?}"
		);
	}
}
