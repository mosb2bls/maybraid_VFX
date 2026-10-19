//! Camera and body ownership. Shake never writes look or locomotion.

use bevy::prelude::*;

/// Pose that samples camera shake and, when no receiver exists, falloff.
///
/// Hosts that overwrite `Transform` each frame (follow cameras) set
/// [`Self::host_rewrites_pose`] so the plugin treats the current translation as
/// rest. Standalone cameras leave it false; the plugin subtracts last frame's
/// offset before applying the next, so overlapping effects cannot drift.
#[derive(Component, Clone, Copy, Debug)]
pub struct SpatialRumbleListener {
	pub gain: f32,
	pub host_rewrites_pose: bool,
}

impl SpatialRumbleListener {
	pub fn new(gain: f32) -> Self {
		Self { gain: gain.max(0.0), host_rewrites_pose: false }
	}

	pub fn rewriting_pose(mut self) -> Self {
		self.host_rewrites_pose = true;
		self
	}
}

impl Default for SpatialRumbleListener {
	fn default() -> Self {
		Self::new(1.0)
	}
}

/// Optional body pose used for falloff and pad sampling.
#[derive(Component, Clone, Copy, Debug)]
pub struct SpatialRumbleReceiver {
	pub gain: f32,
}

impl SpatialRumbleReceiver {
	pub fn new(gain: f32) -> Self {
		Self { gain: gain.max(0.0) }
	}
}

impl Default for SpatialRumbleReceiver {
	fn default() -> Self {
		Self::new(1.0)
	}
}

/// Last camera offset written by the plugin. Restored when the listener is
/// removed so a replacement owner starts from a clean pose.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct AppliedCameraRumble {
	pub translation: Vec3,
	pub roll: f32,
}

impl AppliedCameraRumble {
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	pub fn is_zero(self) -> bool {
		self.translation.length_squared() <= 1e-10 && self.roll.abs() <= 1e-6
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn listener_gain_is_non_negative() {
		let listener = SpatialRumbleListener::new(-2.0);
		assert!(listener.gain.abs() < 1e-5);
		assert!(!listener.host_rewrites_pose);
		assert!(SpatialRumbleListener::new(0.5).rewriting_pose().host_rewrites_pose);
	}

	#[test]
	fn applied_offset_reports_zero() {
		assert!(AppliedCameraRumble::default().is_zero());
		assert!(!AppliedCameraRumble { translation: Vec3::X * 0.1, roll: 0.0 }.is_zero());
	}
}
