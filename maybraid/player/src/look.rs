//! Yaw/pitch helpers shared by camera, item users, and combat aim.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

/// Clearance from vertical gimbal lock used by follow camera and aim pitch clamps.
pub const AIM_PITCH_MARGIN: f32 = 0.1;

/// Map an angle to \((-\pi, \pi]\).
pub fn wrap_pi(angle: f32) -> f32 {
	(angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}

/// Keep pitch inside the same vertical cone as [`crate::PlayerLook`] camera copy.
pub fn clamp_aim_pitch(pitch: f32) -> f32 {
	pitch.clamp(-FRAC_PI_2 + AIM_PITCH_MARGIN, FRAC_PI_2 - AIM_PITCH_MARGIN)
}

/// Yaw delta for look blending: wrap yaw, leave pitch linear.
pub fn look_delta(from: Vec2, to: Vec2) -> Vec2 {
	Vec2::new(wrap_pi(to.x - from.x), to.y - from.y)
}

/// World yaw for a direction projected onto XZ (Bevy +Z forward).
pub fn yaw_xz(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		n.x.atan2(n.z)
	}
}

/// View direction from synthesized look yaw/pitch (camera −Z forward).
pub fn look_forward(look: &crate::PlayerLook) -> Vec3 {
	(Quat::from_rotation_y(look.yaw) * Quat::from_rotation_x(look.pitch)) * -Vec3::Z
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wrap_pi_is_periodic_and_identity_at_zero() {
		let angle = 4.0;
		assert!((wrap_pi(angle) - wrap_pi(angle - std::f32::consts::TAU)).abs() < 1e-5);
		assert!(wrap_pi(0.0).abs() < 1e-5);
		assert!((wrap_pi(std::f32::consts::PI) + std::f32::consts::PI).abs() < 1e-5);
	}

	#[test]
	fn clamp_aim_pitch_matches_camera_margin() {
		let lo = -FRAC_PI_2 + AIM_PITCH_MARGIN;
		let hi = FRAC_PI_2 - AIM_PITCH_MARGIN;
		assert_eq!(clamp_aim_pitch(lo - 1.0), lo);
		assert_eq!(clamp_aim_pitch(hi + 1.0), hi);
	}

	#[test]
	fn look_delta_wraps_yaw_only() {
		let from = Vec2::new(3.0, 0.2);
		let to = Vec2::new(-3.0, 0.5);
		let delta = look_delta(from, to);
		assert!(delta.x.abs() <= std::f32::consts::PI);
		assert_eq!(delta.y, 0.3);
	}

	#[test]
	fn yaw_xz_handles_zero_and_unit_z() {
		assert_eq!(yaw_xz(Vec3::ZERO), 0.0);
		assert!((yaw_xz(Vec3::Z) - 0.0).abs() < 1e-5);
	}

	#[test]
	fn look_forward_points_down_z_at_identity() {
		let look = crate::PlayerLook { yaw: 0.0, pitch: 0.0, first_person: true, focus: 0.0 };
		let forward = look_forward(&look);
		assert!((forward - (-Vec3::Z)).length() < 1e-4);
	}
}
