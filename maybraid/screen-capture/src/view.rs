//! Preferred camera transform, projection, and framing.

use bevy::prelude::{Transform, Vec3};

/// Camera placement and projection for one capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureView {
	pub eye: Vec3,
	pub look_at: Vec3,
	pub up: Vec3,
	pub projection: CaptureProjection,
}

/// Perspective or orthographic capture projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptureProjection {
	Perspective { fov: f32, near: f32, far: f32 },
	Orthographic { scale: f32, near: f32, far: f32 },
}

impl CaptureView {
	pub const DEFAULT_FOV: f32 = 0.75;
	pub const DEFAULT_NEAR: f32 = 0.05;
	pub const DEFAULT_FAR: f32 = 24.0;

	pub fn look_at(eye: Vec3, look_at: Vec3) -> Self {
		Self {
			eye,
			look_at,
			up: Vec3::Y,
			projection: CaptureProjection::Perspective {
				fov: Self::DEFAULT_FOV,
				near: Self::DEFAULT_NEAR,
				far: Self::DEFAULT_FAR,
			},
		}
	}

	pub fn with_up(mut self, up: Vec3) -> Self {
		self.up = up;
		self
	}

	pub fn perspective(mut self, fov: f32) -> Self {
		self.projection = CaptureProjection::Perspective {
			fov,
			near: Self::DEFAULT_NEAR,
			far: Self::DEFAULT_FAR,
		};
		self
	}

	pub fn orthographic(mut self, scale: f32) -> Self {
		self.projection = CaptureProjection::Orthographic {
			scale,
			near: Self::DEFAULT_NEAR,
			far: Self::DEFAULT_FAR,
		};
		self
	}

	pub fn transform(self) -> Transform {
		Transform::from_translation(self.eye).looking_at(self.look_at, self.up)
	}

	/// Canonical cache fragment. Floats use bits so `-0` and `0` stay distinct.
	pub fn cache_fragment(self) -> String {
		format!(
			"eye={}/{}/{} look={}/{}/{} up={}/{}/{} {}",
			self.eye.x.to_bits(),
			self.eye.y.to_bits(),
			self.eye.z.to_bits(),
			self.look_at.x.to_bits(),
			self.look_at.y.to_bits(),
			self.look_at.z.to_bits(),
			self.up.x.to_bits(),
			self.up.y.to_bits(),
			self.up.z.to_bits(),
			self.projection.cache_fragment(),
		)
	}
}

impl CaptureProjection {
	pub fn cache_fragment(self) -> String {
		match self {
			Self::Perspective { fov, near, far } => {
				format!("persp:{}/{}/{}", fov.to_bits(), near.to_bits(), far.to_bits())
			}
			Self::Orthographic { scale, near, far } => {
				format!("ortho:{}/{}/{}", scale.to_bits(), near.to_bits(), far.to_bits())
			}
		}
	}
}

impl Default for CaptureView {
	fn default() -> Self {
		Self::look_at(Vec3::new(1.6, 1.1, 1.6), Vec3::ZERO)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cache_fragment_changes_with_eye() -> anyhow::Result<()> {
		let a = CaptureView::look_at(Vec3::new(1.0, 0.0, 0.0), Vec3::ZERO);
		let b = CaptureView::look_at(Vec3::new(1.0, 0.0, 0.1), Vec3::ZERO);
		assert_ne!(a.cache_fragment(), b.cache_fragment());
		Ok(())
	}

	#[test]
	fn cache_fragment_is_stable() -> anyhow::Result<()> {
		let view = CaptureView::look_at(Vec3::new(0.25, 0.5, 1.0), Vec3::new(0.0, 0.2, 0.0));
		assert_eq!(view.cache_fragment(), view.cache_fragment());
		Ok(())
	}
}
