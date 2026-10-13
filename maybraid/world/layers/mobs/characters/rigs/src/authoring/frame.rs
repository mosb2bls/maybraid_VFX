use bevy::prelude::*;

use super::{CHARACTER_FORWARD, CHARACTER_RIGHT, CHARACTER_UP};

/// Anatomical angles in radians.
///
/// Application order is flexion, then lateral bend, then axial rotation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct JointAngles {
	pub flexion: f32,
	pub lateral: f32,
	pub axial: f32,
}

/// Maps anatomical axes into one bone's local coordinates.
///
/// `anatomy_to_bone` sends anatomical X (flexion), Y (axial), and Z (lateral)
/// onto the bone-local hinge axes. Signs live here when a mirror cannot be a
/// pure rotation. The posed rotation is
///
/// ```text
/// rest * anatomy_to_bone * delta * anatomy_to_bone⁻¹
/// ```
///
/// with `delta = Ry(axial) * Rz(lateral) * Rx(flexion)`. That is a bone-local
/// delta: it is applied before the bind rotation, so a non-identity bind cannot
/// turn a length-axis spin into a bend or the reverse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointFrame {
	pub anatomy_to_bone: Quat,
	pub flexion_sign: f32,
	pub lateral_sign: f32,
	pub axial_sign: f32,
}

impl Default for JointFrame {
	fn default() -> Self {
		Self::IDENTITY
	}
}

impl JointFrame {
	pub const IDENTITY: Self = Self {
		anatomy_to_bone: Quat::IDENTITY,
		flexion_sign: 1.0,
		lateral_sign: 1.0,
		axial_sign: 1.0,
	};

	/// Build a frame whose positive anatomical rotations match the given
	/// **parent-space** axes after `rest` is applied.
	///
	/// Axes should be unit and nearly orthogonal. A reflected triad flips the
	/// axial sign instead of baking a reflection into the quaternion.
	pub fn calibrate(
		rest: Quat,
		flexion_parent: Vec3,
		lateral_parent: Vec3,
		axial_parent: Vec3,
	) -> Option<Self> {
		let into_bone = rest.inverse();
		let flexion = into_bone * flexion_parent;
		let lateral = into_bone * lateral_parent;
		let axial = into_bone * axial_parent;
		Self::from_bone_axes(flexion, lateral, axial)
	}

	pub fn from_bone_axes(flexion: Vec3, lateral: Vec3, axial: Vec3) -> Option<Self> {
		let flexion = flexion.try_normalize()?;
		let lateral = lateral.try_normalize()?;
		let mut axial = axial.try_normalize()?;
		if flexion.dot(lateral).abs() > 0.2
			|| flexion.dot(axial).abs() > 0.2
			|| lateral.dot(axial).abs() > 0.2
		{
			return None;
		}
		let mut axial_sign = 1.0;
		// Right-handed triad: flexion × axial = lateral, up to sign.
		if flexion.cross(axial).dot(lateral) < 0.0 {
			axial = -axial;
			axial_sign = -1.0;
		}
		let basis = Mat3::from_cols(flexion, axial, lateral);
		if !basis.determinant().is_finite() || basis.determinant().abs() < 0.5 {
			return None;
		}
		Some(Self {
			anatomy_to_bone: Quat::from_mat3(&basis).normalize(),
			flexion_sign: 1.0,
			lateral_sign: 1.0,
			axial_sign,
		})
	}

	pub fn with_signs(mut self, flexion: f32, lateral: f32, axial: f32) -> Self {
		self.flexion_sign *= flexion.signum();
		self.lateral_sign *= lateral.signum();
		self.axial_sign *= axial.signum();
		self
	}

	/// Character-space anatomical triad, expressed in this bone's parent rest.
	///
	/// Flexion stays about character +X even when a pelvis or shoulder bind
	/// permutes the parent axes. Identity parent keeps the triad as +X / +Z / +Y.
	pub fn calibrate_in_character(rest: Quat, parent: Quat) -> Option<Self> {
		let into_parent = parent.inverse();
		Self::calibrate(
			rest,
			into_parent * CHARACTER_RIGHT,
			into_parent * CHARACTER_FORWARD,
			into_parent * CHARACTER_UP,
		)
	}

	pub fn local_rotation(&self, rest: Quat, angles: JointAngles) -> Quat {
		let delta = Quat::from_rotation_y(angles.axial * self.axial_sign)
			* Quat::from_rotation_z(angles.lateral * self.lateral_sign)
			* Quat::from_rotation_x(angles.flexion * self.flexion_sign);
		let frame = self.anatomy_to_bone;
		rest * frame * delta * frame.inverse()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	fn sagittal() -> JointFrame {
		JointFrame::calibrate(Quat::IDENTITY, Vec3::X, Vec3::Z, Vec3::Y).expect("orthonormal")
	}

	#[test]
	fn positive_flexion_tips_up_toward_fight_forward() {
		let posed = sagittal()
			.local_rotation(Quat::IDENTITY, JointAngles { flexion: 0.4, lateral: 0.0, axial: 0.0 });
		let tipped = posed * Vec3::Y;
		assert!(tipped.z > 0.2, "expected +Z lean, got {tipped:?}");
		assert!(tipped.x.abs() < 1e-4, "sagittal bend must not yaw, got {tipped:?}");
	}

	#[test]
	fn axial_turn_is_yaw_not_pitch() {
		let posed = sagittal()
			.local_rotation(Quat::IDENTITY, JointAngles { flexion: 0.0, lateral: 0.0, axial: 0.5 });
		let tipped = posed * Vec3::Z;
		assert!(tipped.x.abs() > 0.2, "yaw should move fight-forward sideways, got {tipped:?}");
		assert!(tipped.y.abs() < 1e-4, "yaw must not pitch, got {tipped:?}");
	}

	#[test]
	fn nonidentity_bind_keeps_flexion_in_the_sagittal_plane() {
		// Inspected left femur: local +Y length aims along parent −Z.
		let rest = Quat::from_rotation_x(-FRAC_PI_2);
		let frame = JointFrame::calibrate(rest, Vec3::X, Vec3::Z, Vec3::Y).expect("frame");
		let rest_dir = rest * Vec3::Y;
		assert!(rest_dir.dot(Vec3::NEG_Z) > 0.99, "fixture length {rest_dir:?}");
		let posed =
			frame.local_rotation(rest, JointAngles { flexion: 0.6, lateral: 0.0, axial: 0.0 });
		let dir = posed * Vec3::Y;
		assert!(dir.x.abs() < 0.05, "sagittal flexion must not swing laterally, got {dir:?}");
		assert!((dir - rest_dir).length() > 0.2, "flexion must move the endpoint, got {dir:?}");
		let spun = rest * Quat::from_rotation_y(0.6);
		let spun_dir = spun * Vec3::Y;
		assert!(
			(spun_dir - rest_dir).length() < 0.05,
			"bone-local length spin must not be what flexion does"
		);
	}

	#[test]
	fn zero_angles_preserve_rest() {
		let rest = Quat::from_rotation_z(0.3);
		let frame = JointFrame::calibrate(rest, Vec3::X, Vec3::Z, Vec3::Y).expect("frame");
		let posed = frame.local_rotation(rest, JointAngles::default());
		assert!((posed.dot(rest).abs() - 1.0).abs() < 1e-5);
	}
}
