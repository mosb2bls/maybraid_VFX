//! Bray sonic ring origin: mouth position and horizontal fight-forward.
//!
//! Prefer the assembled Epiphant head rig [`MOUTH_EMIT_SOCKET`] (`nose_socket`) at cue time.
//! When only the quadruped body armature is available (tests, partial rigs), fall back to
//! [`MOUTH_EMIT_NECK_LOCAL`] on the animated `neck` bone.

use bevy::prelude::*;
use character_animations::animations::{MOUTH_EMIT_NECK_LOCAL, MOUTH_EMIT_SOCKET};

use crate::rig::BoneMap;

/// World-space sonic ring spawn frame (horizontal travel along [`Self::forward`]).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BrayMouthEmit {
	pub origin: Vec3,
	/// Unit vector on the XZ plane; rings expand along this axis from [`Self::origin`].
	pub forward: Vec3,
}

impl BrayMouthEmit {
	pub fn from_socket(socket: &GlobalTransform) -> Self {
		Self {
			origin: socket.translation(),
			forward: horizontal_fight_forward(socket),
		}
	}

	pub fn from_neck(neck: &GlobalTransform) -> Self {
		Self {
			origin: neck.transform_point(MOUTH_EMIT_NECK_LOCAL),
			forward: horizontal_fight_forward(neck),
		}
	}
}

/// Project rig +Z fight-forward onto the world XZ plane.
pub fn horizontal_fight_forward(transform: &GlobalTransform) -> Vec3 {
	let forward = transform.rotation() * Vec3::Z;
	let flat = Vec3::new(forward.x, 0.0, forward.z);
	if flat.length_squared() < 1e-8 {
		Vec3::Z
	} else {
		flat.normalize()
	}
}

/// Resolve `nose_socket` on any member rig, else body `neck`.
pub fn sample_bray_mouth_emit(
	bone_maps: &[&BoneMap],
	globals: &Query<&GlobalTransform>,
) -> Option<BrayMouthEmit> {
	for map in bone_maps {
		if let Some(&entity) = map.by_name.get(MOUTH_EMIT_SOCKET) {
			if let Ok(socket) = globals.get(entity) {
				return Some(BrayMouthEmit::from_socket(socket));
			}
		}
	}
	for map in bone_maps {
		if let Some(&entity) = map.by_name.get("neck") {
			if let Ok(neck) = globals.get(entity) {
				return Some(BrayMouthEmit::from_neck(neck));
			}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn horizontal_forward_stays_in_xz() {
		let tf = GlobalTransform::from(
			Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_4)),
		);
		let forward = horizontal_fight_forward(&tf);
		assert!(forward.y.abs() < 1e-6);
		assert!((forward.length() - 1.0).abs() < 1e-5);
	}
}
