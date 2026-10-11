//! Dispatch message, live source, and shared presets.

use bevy::prelude::*;

use crate::channels::{CameraRumbleShape, GroundRumbleShape, PadRumbleShape, RumbleChannels};
use crate::math::{mix64, RumbleEnvelope, RumbleFalloff};

/// World-space rumble any system can dispatch. Bray and heavy landing are
/// presets on this type so they share one API.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SpatialRumble {
	pub origin: Vec3,
	pub intensity: f32,
	pub duration: f32,
	pub envelope: RumbleEnvelope,
	pub falloff: RumbleFalloff,
	pub seed: Option<u64>,
	pub channels: RumbleChannels,
	pub camera: CameraRumbleShape,
	pub pad: PadRumbleShape,
	pub ground: GroundRumbleShape,
}

impl Default for SpatialRumble {
	fn default() -> Self {
		Self {
			origin: Vec3::ZERO,
			intensity: 1.0,
			duration: 0.6,
			envelope: RumbleEnvelope::default(),
			falloff: RumbleFalloff::default(),
			seed: None,
			channels: RumbleChannels::ALL,
			camera: CameraRumbleShape::default(),
			pad: PadRumbleShape::default(),
			ground: GroundRumbleShape::default(),
		}
	}
}

impl SpatialRumble {
	pub fn at(origin: Vec3) -> Self {
		Self { origin, ..Self::default() }
	}

	/// Long-range sonic pulse. Same type as [`Self::heavy_landing`].
	pub fn bray(origin: Vec3) -> Self {
		Self {
			origin,
			intensity: 1.0,
			duration: 1.15,
			envelope: RumbleEnvelope::new(0.08, 0.45),
			falloff: RumbleFalloff::new(6.0, 42.0),
			seed: None,
			channels: RumbleChannels::ALL,
			camera: CameraRumbleShape { translation: 0.11, roll: 0.035, frequency: 9.0 },
			pad: PadRumbleShape { weak: 0.28, strong: 0.62 },
			ground: GroundRumbleShape { amplitude: 1.15, frequency: 6.5 },
		}
	}

	/// Short heavy impact. Same type as [`Self::bray`].
	pub fn heavy_landing(origin: Vec3) -> Self {
		Self {
			origin,
			intensity: 0.9,
			duration: 0.38,
			envelope: RumbleEnvelope::new(0.02, 0.2),
			falloff: RumbleFalloff::new(1.5, 14.0),
			seed: None,
			channels: RumbleChannels::ALL,
			camera: CameraRumbleShape { translation: 0.07, roll: 0.012, frequency: 18.0 },
			pad: PadRumbleShape { weak: 0.22, strong: 0.7 },
			ground: GroundRumbleShape { amplitude: 1.35, frequency: 11.0 },
		}
	}

	pub fn with_seed(mut self, seed: u64) -> Self {
		self.seed = Some(seed);
		self
	}

	pub fn with_intensity(mut self, intensity: f32) -> Self {
		self.intensity = intensity.max(0.0);
		self
	}

	pub fn with_channels(mut self, channels: RumbleChannels) -> Self {
		self.channels = channels;
		self
	}

	pub fn clamped_intensity(self) -> f32 {
		self.intensity.max(0.0)
	}

	pub fn clamped_duration(self) -> f32 {
		self.duration.max(0.0)
	}

	pub fn is_silent(self) -> bool {
		self.clamped_intensity() <= 0.0 || self.clamped_duration() <= 0.0 || !self.channels.any()
	}
}

/// Live instance admitted from [`SpatialRumble`].
#[derive(Component, Clone, Copy, Debug)]
pub struct SpatialRumbleSource {
	pub spec: SpatialRumble,
	pub age: f32,
	pub seed: u64,
}

impl SpatialRumbleSource {
	pub fn from_spec(spec: SpatialRumble, seed: u64) -> Self {
		Self { spec, age: 0.0, seed }
	}

	pub fn remaining(self) -> f32 {
		(self.spec.clamped_duration() - self.age).max(0.0)
	}

	pub fn finished(self) -> bool {
		self.age >= self.spec.clamped_duration()
	}

	pub fn envelope_gain(self) -> f32 {
		self.spec.envelope.sample(self.age, self.spec.clamped_duration())
	}

	pub fn strength_at(self, listener: Vec3) -> f32 {
		let distance = self.spec.origin.distance(listener);
		self.spec.clamped_intensity() * self.envelope_gain() * self.spec.falloff.sample(distance)
	}

	pub fn tick(&mut self, dt: f32) {
		self.age += dt.max(0.0);
	}
}

pub fn resolve_seed(seed: Option<u64>, salt: u64) -> u64 {
	seed.unwrap_or_else(|| mix64(salt ^ 0xD1B5_4A32_C3D2_E1F0))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bray_and_landing_share_the_type_with_different_presets() {
		let bray = SpatialRumble::bray(Vec3::new(1.0, 0.0, 0.0));
		let landing = SpatialRumble::heavy_landing(Vec3::new(1.0, 0.0, 0.0));
		assert_eq!(bray.origin, landing.origin);
		assert!(bray.duration > landing.duration);
		assert!(bray.falloff.outer_radius > landing.falloff.outer_radius);
		assert!(bray.camera.frequency < landing.camera.frequency);
		assert!(bray.channels.camera && landing.channels.ground);
	}

	#[test]
	fn silent_when_intensity_duration_or_channels_are_off() {
		assert!(SpatialRumble::at(Vec3::ZERO).with_intensity(0.0).is_silent());
		assert!(SpatialRumble { duration: 0.0, ..SpatialRumble::default() }.is_silent());
		assert!(SpatialRumble::at(Vec3::ZERO).with_channels(RumbleChannels::none()).is_silent());
		assert!(!SpatialRumble::bray(Vec3::ZERO).is_silent());
	}

	#[test]
	fn strength_scales_with_distance_and_age() {
		let spec = SpatialRumble {
			origin: Vec3::ZERO,
			intensity: 1.0,
			duration: 1.0,
			envelope: RumbleEnvelope::new(0.0, 0.0),
			falloff: RumbleFalloff::new(0.0, 10.0),
			..SpatialRumble::default()
		};
		let live = SpatialRumbleSource::from_spec(spec, 1);
		assert!((live.strength_at(Vec3::ZERO) - 1.0).abs() < 1e-5);
		assert!((live.strength_at(Vec3::X * 5.0) - 0.25).abs() < 1e-5);
		assert!(live.strength_at(Vec3::X * 10.0).abs() < 1e-5);
	}
}
