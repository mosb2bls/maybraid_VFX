//! Apply composed output to camera, ground sample, and pad pulses.

use std::time::Duration;

use bevy::prelude::*;
use maybraid_input::PadRumble;

use crate::compose::{SpatialRumbleOutput, SpatialRumbleSettings};
use crate::listener::{AppliedCameraRumble, SpatialRumbleListener};

/// Time until the next composed pad pulse may be written.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PadRumbleClock {
	pub remaining: f32,
}

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

/// Emit one [`PadRumble`] from the composed, bounded pad channels.
/// Refresh is limited so overlapping sources do not spam `GamepadRumbleRequest`.
pub(crate) fn present_pad_rumble(
	time: Res<Time>,
	output: Res<SpatialRumbleOutput>,
	settings: Res<SpatialRumbleSettings>,
	mut clock: ResMut<PadRumbleClock>,
	mut pad: MessageWriter<PadRumble>,
) {
	clock.remaining -= time.delta_secs().max(0.0);
	if clock.remaining > 0.0 {
		return;
	}
	if let Some(pulse) = pad_pulse_from_output(&output, settings.pad_refresh) {
		pad.write(pulse);
		clock.remaining = settings.pad_refresh.max(1.0 / 60.0);
	}
}

pub fn pad_pulse_from_output(output: &SpatialRumbleOutput, refresh: f32) -> Option<PadRumble> {
	if output.pad_weak <= 0.0 && output.pad_strong <= 0.0 {
		return None;
	}
	let pulse = PadRumble::motors(
		Duration::from_secs_f32(refresh.max(1.0 / 60.0)),
		output.pad_weak,
		output.pad_strong,
	);
	if pulse.is_silent() {
		None
	} else {
		Some(pulse)
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
	use crate::channels::{CameraRumbleShape, PadRumbleShape, RumbleChannels};
	use crate::compose::compose_output;
	use crate::math::{noise, RumbleEnvelope, RumbleFalloff};
	use crate::source::{SpatialRumble, SpatialRumbleSource};

	fn known_source() -> SpatialRumbleSource {
		SpatialRumbleSource::from_spec(
			SpatialRumble {
				origin: Vec3::ZERO,
				intensity: 1.0,
				duration: 2.0,
				envelope: RumbleEnvelope::new(0.0, 0.0),
				falloff: RumbleFalloff::new(0.0, 20.0),
				seed: Some(11),
				channels: RumbleChannels::ALL,
				camera: CameraRumbleShape { translation: 0.2, roll: 0.1, frequency: 0.0 },
				pad: PadRumbleShape { weak: 0.4, strong: 0.8 },
				..SpatialRumble::default()
			}
			.with_seed(11),
			11,
		)
	}

	fn expected_camera_offset(
		source: SpatialRumbleSource,
		settings: &SpatialRumbleSettings,
	) -> Vec3 {
		let ticks = (source.age * source.spec.camera.frequency.max(0.0) * 60.0) as u32;
		Vec3::new(
			noise(source.seed, ticks, 1),
			noise(source.seed, ticks, 2),
			noise(source.seed, ticks, 3),
		) * source.spec.camera.translation
			* source.strength_at(Vec3::ZERO)
			* settings.channel_gain(true, false, false)
	}

	#[test]
	fn listener_gain_is_applied_once_to_a_known_offset() {
		let source = known_source();
		let settings = SpatialRumbleSettings::default();
		let composed = compose_output([&source], Vec3::ZERO, 1.0, &settings);
		let expected = expected_camera_offset(source, &settings);
		assert!((composed.camera_offset - expected).length() < 1e-5);

		let listener = SpatialRumbleListener::new(0.5);
		let mut transform = Transform::from_translation(Vec3::Y);
		let mut applied = AppliedCameraRumble::default();
		apply_camera_shake(&composed, &mut transform, &listener, &mut applied);
		assert!((applied.translation - expected * 0.5).length() < 1e-5);
		assert!((applied.translation - expected * 0.25).length() > 1e-4);
		assert!((transform.translation - (Vec3::Y + expected * 0.5)).length() < 1e-5);
	}

	#[test]
	fn pad_pulse_matches_composed_motors() -> anyhow::Result<()> {
		let output = SpatialRumbleOutput {
			pad_weak: 0.3,
			pad_strong: 0.25,
			..SpatialRumbleOutput::default()
		};
		let pulse = pad_pulse_from_output(&output, 0.0625)
			.ok_or_else(|| anyhow::anyhow!("expected a pulse"))?;
		assert!((pulse.intensity.weak_motor - 0.3).abs() < 1e-5);
		assert!((pulse.intensity.strong_motor - 0.25).abs() < 1e-5);
		assert_eq!(pulse.duration, Duration::from_secs_f32(0.0625));
		assert!(pad_pulse_from_output(&SpatialRumbleOutput::default(), 0.0625).is_none());
		Ok(())
	}

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
