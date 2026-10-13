use bevy::prelude::*;

use crate::RiggedAxis;

/// Local axis along which humanoid bones typically carry their bind length.
pub const BONE_LENGTH_AXIS: Vec3 = Vec3::Y;

/// Directions and elbow flex for a two-segment reach in one coordinate space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoBoneAim {
	pub upper_along: Vec3,
	pub lower_along: Vec3,
	/// Zero when straight; increases as the elbow closes.
	pub flex: f32,
}

impl TwoBoneAim {
	/// Solve a two-segment reach toward `target`, bending toward `pole`.
	///
	/// `target` and `pole` are relative to the upper segment's origin. Targets
	/// outside the reachable annulus are clamped without changing direction.
	pub fn reach(target: Vec3, pole: Vec3, upper_length: f32, lower_length: f32) -> Option<Self> {
		const EPSILON: f32 = 1e-4;
		if upper_length <= EPSILON || lower_length <= EPSILON {
			return None;
		}
		let toward = target.try_normalize()?;
		let pole = (pole - toward * pole.dot(toward)).try_normalize()?;
		let minimum = (upper_length - lower_length).abs() + EPSILON;
		let maximum = upper_length + lower_length - EPSILON;
		let distance = target.length().clamp(minimum, maximum);
		let upper_forward = ((upper_length * upper_length + distance * distance
			- lower_length * lower_length)
			/ (2.0 * upper_length * distance))
			.clamp(-1.0, 1.0);
		let upper_out = (1.0 - upper_forward * upper_forward).sqrt();
		let elbow = toward * (upper_length * upper_forward) + pole * (upper_length * upper_out);
		let target = toward * distance;
		let upper_along = elbow.normalize_or(toward);
		let lower_along = (target - elbow).normalize_or(toward);
		let flex = upper_along.angle_between(lower_along);
		Some(Self { upper_along, lower_along, flex })
	}
}

/// Local bone rotation that aims [`BONE_LENGTH_AXIS`] along `along_parent`, then rolls.
///
/// `along_parent` is the desired length direction in the bone's **parent** space.
/// Aim uses the shortest arc from the rest length direction; `roll` is then applied
/// about the bone's local length axis so it does not disturb the aim.
pub fn rotation_along_with_roll(
	rest: Quat,
	along_parent: Vec3,
	roll: f32,
	length_axis: Vec3,
) -> Quat {
	let Some(along) = along_parent.try_normalize() else {
		return rest * Quat::from_axis_angle(length_axis, roll);
	};
	let rest_along = (rest * length_axis).normalize_or_zero();
	let Some(rest_along) = rest_along.try_normalize() else {
		return rest * Quat::from_axis_angle(length_axis, roll);
	};
	let aim = Quat::from_rotation_arc(rest_along, along);
	// Aim in parent space, then roll about local length (compensated — does not re-aim).
	aim * rest * Quat::from_axis_angle(length_axis, roll)
}

/// Parent-space swing, then twist, then flex, pre-multiplied onto `rest`.
///
/// This is the V0 clip path. The delta axes are constants in the bone's
/// **parent** coordinates: `q = R(swing) * R(twist) * R(flex) * rest`.
pub fn compose_parent_rotation(
	rest: Quat,
	axis: RiggedAxis,
	swing: f32,
	flex: f32,
	twist: f32,
) -> Quat {
	let mut rotation = rest;
	if flex.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.flex_axis, flex) * rotation;
	}
	if twist.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.twist_axis, twist) * rotation;
	}
	if swing.abs() > f32::EPSILON {
		rotation = Quat::from_axis_angle(axis.swing_axis, swing) * rotation;
	}
	rotation
}

/// Convert a world-space displacement into a bone's parent-local space.
///
/// Bevy uses +Y up and −Z forward (glTF +Z forward is flipped on import). Pass
/// [`Vec3::NEG_Y`] scaled by drop magnitude for a world-vertical shift.
pub fn world_displacement_in_parent(parent_world: Quat, world_delta: Vec3) -> Vec3 {
	parent_world.inverse() * world_delta
}

/// Parent-local translation delta that shifts a joint without shortening its bind segment.
///
/// Bones with a non-zero rest segment (e.g. femur `translation.y = 0.25`) should not
/// receive displacement along that segment axis — doing so compresses the bone in the
/// skinned mesh. Joint bones (zero segment) take the full parent-local displacement.
pub fn axis_aware_translation_delta(
	segment_offset: Vec3,
	_axis: RiggedAxis,
	world_delta: Vec3,
	parent_world: Quat,
) -> Vec3 {
	let parent_local = world_displacement_in_parent(parent_world, world_delta);

	if segment_offset.length_squared() <= f32::EPSILON {
		return parent_local;
	}

	let segment_dir = segment_offset.normalize();
	parent_local - segment_dir * parent_local.dot(segment_dir)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	#[test]
	fn compose_parent_rotation_applies_swing_then_flex_order() {
		let rest = Quat::from_rotation_x(0.1);
		let axis = RiggedAxis { swing_axis: Vec3::X, flex_axis: Vec3::Z, twist_axis: Vec3::Y };
		let composed = compose_parent_rotation(rest, axis, 0.3, 0.4, 0.0);
		let expected =
			Quat::from_axis_angle(Vec3::X, 0.3) * Quat::from_axis_angle(Vec3::Z, 0.4) * rest;
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_parent_rotation_applies_flex_twist_swing_order() {
		let rest = Quat::IDENTITY;
		let axis = RiggedAxis { swing_axis: Vec3::Y, flex_axis: Vec3::X, twist_axis: Vec3::Z };
		let composed = compose_parent_rotation(rest, axis, 0.2, 0.3, 0.4);
		let expected = Quat::from_axis_angle(Vec3::Y, 0.2)
			* Quat::from_axis_angle(Vec3::Z, 0.4)
			* Quat::from_axis_angle(Vec3::X, 0.3);
		assert!((composed.dot(expected).abs() - 1.0).abs() < 1e-5);
	}

	#[test]
	fn compose_parent_rotation_with_zero_angles_is_rest() {
		let rest = Quat::from_rotation_y(0.2);
		let composed = compose_parent_rotation(rest, RiggedAxis::DEFAULT, 0.0, 0.0, 0.0);
		assert!((composed.dot(rest).abs() - 1.0).abs() < 1e-6);
	}

	#[test]
	fn rotation_along_with_roll_aims_length_axis() {
		let rest = Quat::IDENTITY;
		let along = Vec3::new(-0.3, -1.0, -0.5).normalize();
		let rot = rotation_along_with_roll(rest, along, 0.0, BONE_LENGTH_AXIS);
		let aimed = (rot * BONE_LENGTH_AXIS).normalize();
		assert!(
			aimed.dot(along) > 0.999,
			"expected length along target, got {aimed:?} vs {along:?}"
		);
	}

	#[test]
	fn rotation_along_with_roll_keeps_aim_when_rolling() {
		let rest = Quat::IDENTITY;
		let along = Vec3::new(0.2, -1.0, -0.4).normalize();
		let rot = rotation_along_with_roll(rest, along, FRAC_PI_2, BONE_LENGTH_AXIS);
		let aimed = (rot * BONE_LENGTH_AXIS).normalize();
		assert!(aimed.dot(along) > 0.999, "roll must not disturb aim, got {aimed:?} vs {along:?}");
	}

	#[test]
	fn two_bone_reach_hits_reachable_target() -> Result<(), &'static str> {
		let target = Vec3::new(0.2, 0.0, 1.2);
		let aim = TwoBoneAim::reach(target, Vec3::NEG_Y, 0.7, 0.7).ok_or("missing reach")?;
		let elbow = aim.upper_along * 0.7;
		let hand = elbow + aim.lower_along * 0.7;
		assert!((hand - target).length() < 1e-4, "{hand:?} vs {target:?}");
		Ok(())
	}

	#[test]
	fn two_bone_reach_clamps_unreachable_target_straight() -> Result<(), &'static str> {
		let aim = TwoBoneAim::reach(Vec3::Z * 10.0, Vec3::NEG_Y, 0.5, 0.5)
			.ok_or("missing clamped reach")?;
		assert!(aim.flex < 0.05, "expected nearly straight reach, got {}", aim.flex);
		Ok(())
	}

	#[test]
	fn axis_aware_delta_preserves_segment_length_on_y_bone() {
		let segment = Vec3::new(0.0, 0.25, 0.0);
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta =
			axis_aware_translation_delta(segment, RiggedAxis::DEFAULT, drop, Quat::IDENTITY);
		assert_eq!(delta, Vec3::ZERO);
	}

	#[test]
	fn axis_aware_delta_shifts_joint_bone_on_world_y() {
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta =
			axis_aware_translation_delta(Vec3::ZERO, RiggedAxis::DEFAULT, drop, Quat::IDENTITY);
		assert_eq!(delta, drop);
	}

	#[test]
	fn axis_aware_delta_converts_world_y_through_parent_rotation() {
		let parent = Quat::from_euler(EulerRot::XYZ, -FRAC_PI_2, 0.0, -FRAC_PI_2);
		let drop = Vec3::new(0.0, -0.15, 0.0);
		let delta = axis_aware_translation_delta(Vec3::ZERO, RiggedAxis::DEFAULT, drop, parent);
		assert!(delta.length() > 0.0);
		assert!((delta.y).abs() < 0.01, "pelvis-local drop should not stay on parent Y");
	}
}
