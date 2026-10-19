//! Admit, tick, bound, and present concurrent sources.

use bevy::prelude::*;

use crate::channels::GroundRumbleSample;
use crate::listener::{AppliedCameraRumble, SpatialRumbleListener, SpatialRumbleReceiver};
use crate::math::noise;
use crate::source::{resolve_seed, SpatialRumble, SpatialRumbleSource};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpatialRumbleSystems {
	Ingest,
	Tick,
	Compose,
	Present,
}

/// Intensity and composition bounds. Gameplay hazards are independent of these.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SpatialRumbleSettings {
	pub master_gain: f32,
	pub camera_gain: f32,
	pub pad_gain: f32,
	pub ground_gain: f32,
	pub max_camera_offset: f32,
	pub max_camera_roll: f32,
	pub max_pad_intensity: f32,
	pub max_ground_amplitude: f32,
	pub max_sources: usize,
	/// Seconds between composed [`PadRumble`] pulses. Keeps `GamepadRumbleRequest`
	/// from stacking every frame while the envelope still updates.
	pub pad_refresh: f32,
}

impl Default for SpatialRumbleSettings {
	fn default() -> Self {
		Self {
			master_gain: 1.0,
			camera_gain: 1.0,
			pad_gain: 1.0,
			ground_gain: 1.0,
			max_camera_offset: 0.22,
			max_camera_roll: 0.06,
			max_pad_intensity: 1.0,
			max_ground_amplitude: 1.5,
			max_sources: 8,
			pad_refresh: DEFAULT_PAD_REFRESH,
		}
	}
}

pub const DEFAULT_PAD_REFRESH: f32 = 1.0 / 16.0;

impl SpatialRumbleSettings {
	pub fn silent() -> Self {
		Self { master_gain: 0.0, ..Self::default() }
	}

	pub fn channel_gain(self, camera: bool, pad: bool, ground: bool) -> f32 {
		let channel = if camera {
			self.camera_gain
		} else if pad {
			self.pad_gain
		} else if ground {
			self.ground_gain
		} else {
			0.0
		};
		(self.master_gain * channel).max(0.0)
	}
}

/// Bounded presentation written each compose tick.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct SpatialRumbleOutput {
	pub camera_offset: Vec3,
	pub camera_roll: f32,
	pub pad_weak: f32,
	pub pad_strong: f32,
	pub ground: GroundRumbleSample,
}

impl SpatialRumbleOutput {
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	pub fn is_quiet(self) -> bool {
		self.camera_offset.length_squared() <= 1e-10
			&& self.camera_roll.abs() <= 1e-6
			&& self.pad_weak <= 0.0
			&& self.pad_strong <= 0.0
			&& self.ground.is_silent()
	}
}

impl Default for SpatialRumbleOutput {
	fn default() -> Self {
		Self {
			camera_offset: Vec3::ZERO,
			camera_roll: 0.0,
			pad_weak: 0.0,
			pad_strong: 0.0,
			ground: GroundRumbleSample::silent(),
		}
	}
}

pub(crate) fn ingest_rumbles(
	mut commands: Commands,
	mut messages: MessageReader<SpatialRumble>,
	settings: Res<SpatialRumbleSettings>,
	existing: Query<(Entity, &SpatialRumbleSource)>,
) {
	let mut live: Vec<(Entity, f32)> =
		existing.iter().map(|(entity, source)| (entity, source.age)).collect();
	live.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
	let mut admitted = live.len();
	let mut salt = 1u64;
	for spec in messages.read() {
		if spec.is_silent() {
			continue;
		}
		while admitted >= settings.max_sources {
			if let Some((entity, _)) = live.pop() {
				commands.entity(entity).try_despawn();
				admitted = admitted.saturating_sub(1);
			} else {
				break;
			}
		}
		if admitted >= settings.max_sources {
			break;
		}
		salt = salt.wrapping_add(1);
		let seed = resolve_seed(spec.seed, salt);
		commands.spawn(SpatialRumbleSource::from_spec(*spec, seed));
		admitted += 1;
	}
}

pub(crate) fn tick_rumbles(
	mut commands: Commands,
	time: Res<Time>,
	mut sources: Query<(Entity, &mut SpatialRumbleSource)>,
) {
	let dt = time.delta_secs();
	for (entity, mut source) in &mut sources {
		source.tick(dt);
		if source.finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

pub(crate) fn compose_rumbles(
	settings: Res<SpatialRumbleSettings>,
	sources: Query<&SpatialRumbleSource>,
	receivers: Query<(&GlobalTransform, &SpatialRumbleReceiver)>,
	listeners: Query<(&GlobalTransform, &SpatialRumbleListener, Option<&AppliedCameraRumble>)>,
	mut output: ResMut<SpatialRumbleOutput>,
) {
	// Listener gain is camera presentation only and is applied once in
	// [`crate::present::apply_camera_shake`]. A body receiver may still scale
	// compose; a listener-only host samples at the camera rest pose with gain 1.
	let (sample_at, receiver_gain) = if let Some((transform, receiver)) = receivers.iter().next() {
		(transform.translation(), receiver.gain)
	} else if let Some((transform, listener, applied)) = listeners.iter().next() {
		(rest_translation(transform.translation(), listener, applied), 1.0)
	} else {
		(Vec3::ZERO, 1.0)
	};
	*output = compose_output(sources.iter(), sample_at, receiver_gain, &settings);
}

/// Compose live sources at `sample_at` into a bounded output.
pub fn compose_output<'a>(
	sources: impl IntoIterator<Item = &'a SpatialRumbleSource>,
	sample_at: Vec3,
	receiver_gain: f32,
	settings: &SpatialRumbleSettings,
) -> SpatialRumbleOutput {
	let mut camera_offset = Vec3::ZERO;
	let mut camera_roll = 0.0;
	let mut pad_weak = 0.0;
	let mut pad_strong = 0.0;
	let mut ground_amp_sq = 0.0;
	let mut ground_freq_weight = 0.0;
	let mut ground_dir = Vec3::ZERO;
	let mut ground_origin = Vec3::ZERO;
	let mut ground_weight = 0.0;
	for source in sources {
		let strength = source.strength_at(sample_at) * receiver_gain.max(0.0);
		if strength <= 0.0 {
			continue;
		}
		let ticks = (source.age * source.spec.camera.frequency.max(0.0) * 60.0) as u32;
		if source.spec.channels.camera {
			let gain = strength * settings.channel_gain(true, false, false);
			camera_offset += Vec3::new(
				noise(source.seed, ticks, 1),
				noise(source.seed, ticks, 2),
				noise(source.seed, ticks, 3),
			) * source.spec.camera.translation
				* gain;
			camera_roll += noise(source.seed, ticks, 4) * source.spec.camera.roll * gain;
		}
		if source.spec.channels.pad {
			let gain = strength * settings.channel_gain(false, true, false);
			pad_weak += source.spec.pad.weak * gain;
			pad_strong += source.spec.pad.strong * gain;
		}
		if source.spec.channels.ground {
			let gain = strength * settings.channel_gain(false, false, true);
			let amplitude = source.spec.ground.amplitude * gain;
			ground_amp_sq += amplitude * amplitude;
			ground_freq_weight += source.spec.ground.frequency * amplitude.abs();
			let planar = planar_away(source.spec.origin, sample_at);
			ground_dir += planar * amplitude.abs();
			ground_origin += source.spec.origin * amplitude.abs();
			ground_weight += amplitude.abs();
		}
	}
	let camera_offset = clamp_vec(camera_offset, settings.max_camera_offset);
	let camera_roll = camera_roll.clamp(-settings.max_camera_roll, settings.max_camera_roll);
	let pad_limit = settings.max_pad_intensity.max(0.0);
	SpatialRumbleOutput {
		camera_offset,
		camera_roll,
		pad_weak: pad_weak.clamp(0.0, pad_limit),
		pad_strong: pad_strong.clamp(0.0, pad_limit),
		ground: ground_sample(
			ground_amp_sq,
			ground_freq_weight,
			ground_dir,
			ground_origin,
			ground_weight,
			settings.max_ground_amplitude,
		),
	}
}

fn ground_sample(
	amp_sq: f32,
	freq_weight: f32,
	dir: Vec3,
	origin: Vec3,
	weight: f32,
	max_amplitude: f32,
) -> GroundRumbleSample {
	if weight <= 0.0 {
		return GroundRumbleSample::silent();
	}
	GroundRumbleSample {
		amplitude: amp_sq.sqrt().min(max_amplitude.max(0.0)),
		frequency: freq_weight / weight,
		direction: if dir.length_squared() > 1e-8 { dir.normalize() } else { Vec3::ZERO },
		origin: origin / weight,
	}
}

fn clamp_vec(offset: Vec3, max: f32) -> Vec3 {
	let max = max.max(0.0);
	let length = offset.length();
	if length > max && length > 0.0 {
		offset * (max / length)
	} else {
		offset
	}
}

fn planar_away(origin: Vec3, sample_at: Vec3) -> Vec3 {
	let mut delta = sample_at - origin;
	delta.y = 0.0;
	if delta.length_squared() > 1e-8 {
		delta.normalize()
	} else {
		Vec3::ZERO
	}
}

fn rest_translation(
	translation: Vec3,
	listener: &SpatialRumbleListener,
	applied: Option<&AppliedCameraRumble>,
) -> Vec3 {
	if listener.host_rewrites_pose {
		translation
	} else if let Some(applied) = applied {
		translation - applied.translation
	} else {
		translation
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::channels::{CameraRumbleShape, RumbleChannels};
	use crate::math::{RumbleEnvelope, RumbleFalloff};

	fn hold_source(origin: Vec3, seed: u64) -> SpatialRumbleSource {
		SpatialRumbleSource::from_spec(
			SpatialRumble {
				origin,
				intensity: 1.0,
				duration: 1.0,
				envelope: RumbleEnvelope::new(0.0, 0.0),
				falloff: RumbleFalloff::new(0.0, 20.0),
				seed: Some(seed),
				channels: RumbleChannels::ALL,
				camera: CameraRumbleShape { translation: 0.2, roll: 0.1, frequency: 0.0 },
				..SpatialRumble::default()
			}
			.with_seed(seed),
			seed,
		)
	}

	#[test]
	fn falloff_reaches_compose() {
		let settings = SpatialRumbleSettings::default();
		let source = hold_source(Vec3::ZERO, 3);
		let near = compose_output([&source], Vec3::ZERO, 1.0, &settings);
		let far = compose_output([&source], Vec3::X * 20.0, 1.0, &settings);
		assert!(near.camera_offset.length() > 0.0);
		assert!(far.is_quiet());
		assert!(near.ground.amplitude > far.ground.amplitude);
	}

	#[test]
	fn stacking_is_bounded() {
		let settings = SpatialRumbleSettings {
			max_camera_offset: 0.05,
			max_ground_amplitude: 0.4,
			max_pad_intensity: 0.3,
			..SpatialRumbleSettings::default()
		};
		let a = hold_source(Vec3::ZERO, 1);
		let b = hold_source(Vec3::ZERO, 2);
		let one = compose_output([&a], Vec3::ZERO, 1.0, &settings);
		let two = compose_output([&a, &b], Vec3::ZERO, 1.0, &settings);
		assert!(two.camera_offset.length() <= settings.max_camera_offset + 1e-5);
		assert!(two.ground.amplitude <= settings.max_ground_amplitude + 1e-5);
		assert!(two.pad_strong <= settings.max_pad_intensity + 1e-5);
		assert!(two.camera_offset.length() + 1e-5 >= one.camera_offset.length().min(0.05));
	}

	#[test]
	fn master_gain_silences_every_channel() {
		let source = hold_source(Vec3::ZERO, 4);
		let output = compose_output([&source], Vec3::ZERO, 1.0, &SpatialRumbleSettings::silent());
		assert!(output.is_quiet());
	}

	#[test]
	fn overlapping_sources_share_caps_and_envelopes_on_all_channels() {
		let settings = SpatialRumbleSettings {
			max_camera_offset: 0.05,
			max_camera_roll: 0.02,
			max_ground_amplitude: 0.4,
			max_pad_intensity: 0.3,
			..SpatialRumbleSettings::default()
		};
		let mut attack = hold_source(Vec3::ZERO, 1);
		attack.spec.envelope = RumbleEnvelope::new(0.4, 0.0);
		attack.spec.duration = 1.0;
		let mut peak = hold_source(Vec3::ZERO, 2);
		peak.spec.envelope = RumbleEnvelope::new(0.0, 0.0);
		let early = compose_output([&attack, &peak], Vec3::ZERO, 1.0, &settings);
		attack.age = 0.4;
		let late = compose_output([&attack, &peak], Vec3::ZERO, 1.0, &settings);
		assert!(early.camera_offset.length() <= settings.max_camera_offset + 1e-5);
		assert!(late.camera_offset.length() <= settings.max_camera_offset + 1e-5);
		assert!(early.camera_roll.abs() <= settings.max_camera_roll + 1e-5);
		assert!(late.pad_weak <= settings.max_pad_intensity + 1e-5);
		assert!(late.pad_strong <= settings.max_pad_intensity + 1e-5);
		assert!(late.ground.amplitude <= settings.max_ground_amplitude + 1e-5);
		assert!(late.pad_strong + 1e-5 >= early.pad_strong);
		assert!(late.ground.amplitude + 1e-5 >= early.ground.amplitude);
		let pulse = crate::pad_pulse_from_output(&late, settings.pad_refresh);
		assert!(pulse.is_some());
		if let Some(pulse) = pulse {
			assert!(pulse.intensity.weak_motor <= settings.max_pad_intensity + 1e-5);
			assert!(pulse.intensity.strong_motor <= settings.max_pad_intensity + 1e-5);
		}
	}
}
