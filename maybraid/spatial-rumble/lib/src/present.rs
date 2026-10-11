//! Apply composed output to camera, ground sample, and pad pulses.

use bevy::prelude::*;

use crate::compose::SpatialRumbleOutput;
use crate::listener::{AppliedCameraRumble, SpatialRumbleListener};

pub(crate) fn present_camera_rumble(
	mut commands: Commands,
	output: Res<SpatialRumbleOutput>,
	mut cameras: Query<(
		Entity,
		&mut Transform,
		&SpatialRumbleListener,
		Option<&mut AppliedCameraRumble>,
	)>,
) {
	for (entity, mut transform, listener, applied) in &mut cameras {
		let mut next = applied.as_deref().copied().unwrap_or_default();
		apply_camera_shake(&output, &mut transform, listener, &mut next);
		if applied.is_none() {
			commands.entity(entity).insert(next);
		} else if let Some(mut applied) = applied {
			*applied = next;
		}
	}
}

pub(crate) fn cleanup_removed_listeners(
	mut commands: Commands,
	mut removed: RemovedComponents<SpatialRumbleListener>,
	mut cameras: Query<(&mut Transform, Option<&mut AppliedCameraRumble>)>,
	mut output: ResMut<SpatialRumbleOutput>,
) {
	let mut replaced = false;
	for entity in removed.read() {
		replaced = true;
		let Ok((mut transform, applied)) = cameras.get_mut(entity) else {
			continue;
		};
		if let Some(mut applied) = applied {
			transform.translation -= applied.translation;
			transform.rotation *= Quat::from_rotation_z(-applied.roll);
			applied.clear();
			commands.entity(entity).remove::<AppliedCameraRumble>();
		}
	}
	if replaced {
		output.clear();
	}
}

/// Write the composed offset without accumulating onto last frame's shake.
pub fn apply_camera_shake(
	output: &SpatialRumbleOutput,
	transform: &mut Transform,
	listener: &SpatialRumbleListener,
	applied: &mut AppliedCameraRumble,
) {
	let rest = if listener.host_rewrites_pose {
		transform.translation
	} else {
		transform.translation - applied.translation
	};
	let rest_rotation = if listener.host_rewrites_pose {
		transform.rotation
	} else {
		transform.rotation * Quat::from_rotation_z(-applied.roll)
	};
	let offset = output.camera_offset * listener.gain;
	let roll = output.camera_roll * listener.gain;
	transform.translation = rest + offset;
	transform.rotation = rest_rotation * Quat::from_rotation_z(roll);
	applied.translation = offset;
	applied.roll = roll;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shake_does_not_accumulate_on_a_standalone_camera() {
		let listener = SpatialRumbleListener::new(1.0);
		let mut transform = Transform::from_translation(Vec3::Y);
		let mut applied = AppliedCameraRumble::default();
		let output = SpatialRumbleOutput {
			camera_offset: Vec3::X * 0.1,
			camera_roll: 0.02,
			..SpatialRumbleOutput::default()
		};
		apply_camera_shake(&output, &mut transform, &listener, &mut applied);
		apply_camera_shake(&output, &mut transform, &listener, &mut applied);
		assert!((transform.translation - (Vec3::Y + Vec3::X * 0.1)).length() < 1e-5);
		assert!((applied.translation - Vec3::X * 0.1).length() < 1e-5);
	}

	#[test]
	fn rewriting_host_treats_current_pose_as_rest() {
		let listener = SpatialRumbleListener::new(1.0).rewriting_pose();
		let mut transform = Transform::from_translation(Vec3::new(3.0, 1.0, 0.0));
		let mut applied = AppliedCameraRumble { translation: Vec3::X * 9.0, roll: 0.4 };
		let output =
			SpatialRumbleOutput { camera_offset: Vec3::Z * 0.05, ..SpatialRumbleOutput::default() };
		apply_camera_shake(&output, &mut transform, &listener, &mut applied);
		assert!((transform.translation - Vec3::new(3.0, 1.0, 0.05)).length() < 1e-5);
	}

	#[test]
	fn finished_output_restores_the_rest_pose() {
		let listener = SpatialRumbleListener::new(1.0);
		let rest = Transform::from_xyz(2.0, 1.5, -1.0);
		let mut transform = rest;
		let mut applied = AppliedCameraRumble::default();
		let active = SpatialRumbleOutput {
			camera_offset: Vec3::new(0.04, 0.02, -0.03),
			camera_roll: 0.01,
			..SpatialRumbleOutput::default()
		};
		apply_camera_shake(&active, &mut transform, &listener, &mut applied);
		apply_camera_shake(
			&SpatialRumbleOutput::default(),
			&mut transform,
			&listener,
			&mut applied,
		);
		assert!((transform.translation - rest.translation).length() < 1e-5);
		assert!(applied.is_zero());
	}
}
