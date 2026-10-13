//! Item-use arm reach overlays: body-space targets, keyframe tracks, and humanoid IK apply.
//!
//! # Body space (documented item convention)
//!
//! Reach targets and elbow poles are authored in **body space** relative to the upper arm:
//! - **+X** = the character's right
//! - **+Y** = up
//! - **+Z** = fight-forward (ahead of the torso)
//!
//! Humanoid v0 shoulder bind frames flip lateral X ([`BodyReachSpace::to_shoulder`]); callers
//! should never negate X themselves.

use bevy::prelude::*;

use crate::articulation::{
	compose_parent_rotation, rotation_along_with_roll, TwoBoneAim, BONE_LENGTH_AXIS,
};
use crate::authoring::humanoid_bone_axis;
use crate::rigs::humanoid_v0::HumanoidV0Rig;
use crate::Side;

pub mod overhand_throw;

pub use overhand_throw::OverhandThrow;

/// Documented body-space reach axes for item overlays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BodyReachSpace;

impl BodyReachSpace {
	/// Convert a body-space offset into humanoid v0 upper-segment (shoulder) space.
	///
	/// On this rig, bind **+X points left**; documented **+X is right**, so lateral X is negated
	/// per [`Side`].
	pub fn to_shoulder(side: Side, body: Vec3) -> Vec3 {
		Vec3::new(side.sign() * body.x, body.y, body.z)
	}

	/// Inverse of [`Self::to_shoulder`].
	pub fn from_shoulder(side: Side, shoulder: Vec3) -> Vec3 {
		Vec3::new(side.sign() * shoulder.x, shoulder.y, shoulder.z)
	}
}

/// Elbow pole hint in body space, with a fallback when the primary plane is degenerate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmReachPole {
	pub primary: Vec3,
	pub fallback: Vec3,
}

impl ArmReachPole {
	pub const fn new(primary: Vec3, fallback: Vec3) -> Self {
		Self { primary, fallback }
	}

	fn to_shoulder(self, side: Side) -> (Vec3, Vec3) {
		(
			BodyReachSpace::to_shoulder(side, self.primary),
			BodyReachSpace::to_shoulder(side, self.fallback),
		)
	}
}

/// Piecewise-linear reach track in body space.
#[derive(Debug, Clone, PartialEq)]
pub struct ArmReachTrack {
	pub keyframes: Vec<(f32, Vec3)>,
}

impl ArmReachTrack {
	pub fn from_keyframes(keyframes: &[(f32, Vec3)]) -> Self {
		Self { keyframes: keyframes.to_vec() }
	}

	/// Sample reach at normalized time `t` in `[0, 1]`.
	pub fn sample(&self, t: f32) -> Vec3 {
		let t = t.clamp(0.0, 1.0);
		let keyframes = &self.keyframes;
		if keyframes.is_empty() {
			return Vec3::ZERO;
		}
		for window in keyframes.windows(2) {
			let (t0, a) = window[0];
			let (t1, b) = window[1];
			if t <= t1 {
				let u = ((t - t0) / (t1 - t0).max(1e-4)).clamp(0.0, 1.0);
				return a.lerp(b, u);
			}
		}
		keyframes[keyframes.len() - 1].1
	}
}

/// Optional post-sample clamp in body space (keeps reach in a safe hull).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmReachClamp {
	/// Minimum body-space +X (right); `None` skips.
	pub min_x: Option<f32>,
	/// Minimum body-space +Z (forward); `None` skips.
	pub min_z: Option<f32>,
}

impl ArmReachClamp {
	pub fn apply(self, mut reach: Vec3) -> Vec3 {
		if let Some(min_x) = self.min_x {
			reach.x = reach.x.max(min_x);
		}
		if let Some(min_z) = self.min_z {
			reach.z = reach.z.max(min_z);
		}
		reach
	}
}

/// Two-bone IK reach solve + apply for a humanoid arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HumanoidArmReach;

impl HumanoidArmReach {
	/// Solve reach toward `target` (body space) from the given rig segment lengths.
	pub fn solve(
		rig: &HumanoidV0Rig,
		side: Side,
		target: Vec3,
		pole: ArmReachPole,
	) -> Option<TwoBoneAim> {
		let (_, forearm_name) = arm_bone_names(side);
		let id = rig.binding.definition.id(forearm_name)?;
		let length = rig.binding.effective_rest.get(id)?.translation.length();
		let target = BodyReachSpace::to_shoulder(side, target);
		let (primary, fallback) = pole.to_shoulder(side);
		TwoBoneAim::reach(target, primary, length, length)
			.or_else(|| TwoBoneAim::reach(target, fallback, length, length))
	}

	/// Write a solved reach onto `rig` (humerus aim + forearm flex + humerus roll).
	pub fn apply(rig: &mut HumanoidV0Rig, side: Side, reach: TwoBoneAim) {
		let (humerus_name, forearm_name) = arm_bone_names(side);
		let Some(humerus) = rig.binding.definition.id(humerus_name) else {
			return;
		};
		let Some(forearm) = rig.binding.definition.id(forearm_name) else {
			return;
		};
		let roll = Self::humerus_roll_for_reach(rig, side, reach);
		let rest_humerus = rig.binding.effective_rest.rotation(humerus);
		let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
		let aimed = rotation_along_with_roll(
			rest_humerus,
			parent.inverse() * reach.upper_along,
			roll,
			BONE_LENGTH_AXIS,
		);
		rig.pose.set_rotation(humerus, aimed);
		let flexed = compose_parent_rotation(
			rig.binding.effective_rest.rotation(forearm),
			humanoid_bone_axis(forearm_name),
			0.0,
			reach.flex,
			0.0,
		);
		rig.pose.set_rotation(forearm, flexed);
	}

	fn humerus_roll_for_reach(rig: &HumanoidV0Rig, side: Side, reach: TwoBoneAim) -> f32 {
		let (humerus_name, forearm_name) = arm_bone_names(side);
		let Some(humerus) = rig.binding.definition.id(humerus_name) else {
			return 0.0;
		};
		let Some(forearm) = rig.binding.definition.id(forearm_name) else {
			return 0.0;
		};
		let parent = rig.binding.definition.parent_rotation(&rig.pose, humerus);
		let aimed = rotation_along_with_roll(
			rig.binding.effective_rest.rotation(humerus),
			parent.inverse() * reach.upper_along,
			0.0,
			BONE_LENGTH_AXIS,
		);
		let humerus_world = parent * aimed;
		let forearm_rotation = compose_parent_rotation(
			rig.binding.effective_rest.rotation(forearm),
			humanoid_bone_axis(forearm_name),
			0.0,
			reach.flex,
			0.0,
		);
		let zero_roll_lower = forearm_rotation * BONE_LENGTH_AXIS;
		let desired_lower = humerus_world.inverse() * reach.lower_along;
		signed_angle_about_axis(zero_roll_lower, desired_lower, BONE_LENGTH_AXIS).unwrap_or(0.0)
	}
}

fn arm_bone_names(side: Side) -> (&'static str, &'static str) {
	match side {
		Side::Left => ("humerus.L", "forearm.L"),
		Side::Right => ("humerus.R", "forearm.R"),
	}
}

fn signed_angle_about_axis(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
	let axis = axis.try_normalize()?;
	let from = (from - axis * from.dot(axis)).try_normalize()?;
	let to = (to - axis * to.dot(axis)).try_normalize()?;
	Some(axis.dot(from.cross(to)).atan2(from.dot(to)))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::rigs::humanoid_v0::HumanoidV0Rig;

	fn rig_with_arm_lengths(length: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::imported();
		rig.seed_rest("humerus.R", Transform::from_translation(Vec3::Y * length));
		rig.seed_rest("forearm.R", Transform::from_translation(Vec3::Y * length));
		rig.pose.copy_from(&rig.binding.effective_rest);
		rig
	}

	#[test]
	fn body_space_negates_lateral_x_for_right_shoulder() -> anyhow::Result<()> {
		let body = Vec3::new(0.12, 0.2, 0.4);
		let shoulder = BodyReachSpace::to_shoulder(Side::Right, body);
		assert!(shoulder.x < 0.0, "rig +X is left, so right reach is −X, got {shoulder:?}");
		assert_eq!(shoulder.y, body.y);
		assert_eq!(shoulder.z, body.z);
		let round_trip = BodyReachSpace::from_shoulder(Side::Right, shoulder);
		assert!((round_trip - body).length() < 1e-5);
		Ok(())
	}

	#[test]
	fn track_samples_between_keyframes() -> anyhow::Result<()> {
		let track = ArmReachTrack::from_keyframes(&[
			(0.0, Vec3::new(0.0, 0.0, 0.0)),
			(1.0, Vec3::new(1.0, 0.0, 0.0)),
		]);
		let mid = track.sample(0.5);
		assert!((mid.x - 0.5).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn solve_reach_wings_right_elbow_out() -> anyhow::Result<()> {
		let rig = rig_with_arm_lengths(0.35);
		let target = Vec3::new(0.10, 0.14, 0.42);
		let pole = OverhandThrow::pole();
		let reach = HumanoidArmReach::solve(&rig, Side::Right, target, pole)
			.ok_or_else(|| anyhow::anyhow!("missing reach"))?;
		assert!(
			reach.upper_along.x < 0.0,
			"elbow must wing out laterally, got {:?}",
			reach.upper_along
		);
		Ok(())
	}

	#[test]
	fn apply_reach_poses_humerus_and_forearm() -> anyhow::Result<()> {
		let mut rig = rig_with_arm_lengths(0.35);
		let target = Vec3::new(0.10, 0.14, 0.42);
		let reach = HumanoidArmReach::solve(&rig, Side::Right, target, OverhandThrow::pole())
			.ok_or_else(|| anyhow::anyhow!("missing reach"))?;
		HumanoidArmReach::apply(&mut rig, Side::Right, reach);
		assert_ne!(rig.rotation("humerus.R"), Quat::IDENTITY);
		Ok(())
	}
}
