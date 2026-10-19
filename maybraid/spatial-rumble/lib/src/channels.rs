//! Presentation channels. Ground is a sample contract, not mesh displacement.

use bevy::prelude::*;

/// Which presentations a source may drive. Gameplay hazards are not a channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RumbleChannels {
	pub camera: bool,
	pub pad: bool,
	pub ground: bool,
}

impl RumbleChannels {
	pub const ALL: Self = Self { camera: true, pad: true, ground: true };
	pub const CAMERA: Self = Self { camera: true, pad: false, ground: false };
	pub const GROUND: Self = Self { camera: false, pad: false, ground: true };
	pub const PAD: Self = Self { camera: false, pad: true, ground: false };

	pub fn none() -> Self {
		Self { camera: false, pad: false, ground: false }
	}

	pub fn any(self) -> bool {
		self.camera || self.pad || self.ground
	}
}

impl Default for RumbleChannels {
	fn default() -> Self {
		Self::ALL
	}
}

/// Per-frame ground presentation. Consumers may drive dust, rings, or material
/// motion from this sample. This crate never displaces terrain geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundRumbleSample {
	pub amplitude: f32,
	pub frequency: f32,
	pub direction: Vec3,
	pub origin: Vec3,
}

impl GroundRumbleSample {
	pub fn silent() -> Self {
		Self { amplitude: 0.0, frequency: 0.0, direction: Vec3::ZERO, origin: Vec3::ZERO }
	}

	pub fn is_silent(self) -> bool {
		self.amplitude <= 0.0
	}
}

impl Default for GroundRumbleSample {
	fn default() -> Self {
		Self::silent()
	}
}

/// How camera shake is shaped at intensity `1` beside the source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraRumbleShape {
	pub translation: f32,
	pub roll: f32,
	pub frequency: f32,
}

impl Default for CameraRumbleShape {
	fn default() -> Self {
		Self { translation: 0.08, roll: 0.02, frequency: 14.0 }
	}
}

/// Weak / strong motor weights at intensity `1` beside the source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadRumbleShape {
	pub weak: f32,
	pub strong: f32,
}

impl Default for PadRumbleShape {
	fn default() -> Self {
		Self { weak: 0.35, strong: 0.55 }
	}
}

/// Ground sample weights at intensity `1` beside the source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundRumbleShape {
	pub amplitude: f32,
	pub frequency: f32,
}

impl Default for GroundRumbleShape {
	fn default() -> Self {
		Self { amplitude: 1.0, frequency: 8.0 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn all_enables_every_presentation() {
		let all = RumbleChannels { camera: true, pad: true, ground: true };
		assert!(all.any());
		assert_eq!(all, RumbleChannels::ALL);
		assert!(!RumbleChannels::none().any());
	}

	#[test]
	fn silent_ground_has_no_amplitude() {
		assert!(GroundRumbleSample::silent().is_silent());
	}
}
