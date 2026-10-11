//! Preferred camera transform, projection, and framing.

use bevy::math::bounding::Aabb3d;
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

/// How the camera uses subject extents after the scene is ready.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CaptureFraming {
	/// Keep the authored [`CaptureView`].
	Authored,
	/// Recenter and zoom so the mesh AABB fills the frame.
	FitExtents { padding: f32 },
}

impl CaptureFraming {
	/// Tight icon fill. HUD slots stay compact; this is the crop, not the tile.
	pub const ICON: Self = Self::FitExtents { padding: 0.05 };

	pub fn cache_fragment(self) -> String {
		match self {
			Self::Authored => String::from("authored"),
			Self::FitExtents { padding } => format!("fit:{}", padding.to_bits()),
		}
	}
}

impl Default for CaptureFraming {
	fn default() -> Self {
		Self::Authored
	}
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

	/// Zoom so `aabb` fills the frame.
	///
	/// Keeps the authored look-at height (inspect receiver). Trucks on the
	/// camera-right axis so the *projected* box is centered in the image — a
	/// 3/4 view does not put the 3D AABB center at NDC x = 0.
	pub fn fit_extents(self, aabb: Aabb3d, aspect: f32, padding: f32) -> Self {
		let aabb_center = (Vec3::from(aabb.min) + Vec3::from(aabb.max)) * 0.5;
		let mut forward = self.look_at - self.eye;
		if forward.length_squared() < 1e-8 {
			forward = Vec3::new(-1.0, -0.35, -1.0);
		}
		let forward = forward.normalize();
		let mut right = forward.cross(self.up);
		if right.length_squared() < 1e-8 {
			right = forward.cross(Vec3::Y);
		}
		let right = right.normalize();
		let up = right.cross(forward).normalize();
		let mut look_at = Vec3::new(aabb_center.x, self.look_at.y, aabb_center.z);
		match self.projection {
			CaptureProjection::Perspective { fov, near, .. } => {
				let mut dist = perspective_fit_distance(
					aabb, look_at, forward, right, up, fov, aspect, padding, near,
				);
				for _ in 0..4 {
					let (min_x, max_x) = projected_tan_x(aabb, look_at, forward, right, dist);
					let mid_x = (min_x + max_x) * 0.5;
					if mid_x.abs() < 1e-5 {
						break;
					}
					look_at += right * (mid_x * dist);
					look_at.y = self.look_at.y;
					dist = perspective_fit_distance(
						aabb, look_at, forward, right, up, fov, aspect, padding, near,
					);
				}
				Self { eye: look_at - forward * dist, look_at, up, projection: self.projection }
			}
			CaptureProjection::Orthographic { near, far, .. } => {
				let mut min_x = f32::MAX;
				let mut max_x = f32::MIN;
				for corner in aabb_corners(aabb) {
					let x = (corner - look_at).dot(right);
					min_x = min_x.min(x);
					max_x = max_x.max(x);
				}
				look_at += right * ((min_x + max_x) * 0.5);
				look_at.y = self.look_at.y;
				Self {
					eye: look_at - forward * (near + (far - near) * 0.35).max(1.0),
					look_at,
					up,
					projection: CaptureProjection::Orthographic {
						scale: orthographic_fit_scale(aabb, look_at, right, up, aspect, padding),
						near,
						far,
					},
				}
			}
		}
	}

	/// Keep the authored look-at height; shift the eye by the same delta.
	pub fn with_locked_look_at_y(self, y: f32) -> Self {
		let delta = self.look_at.y - y;
		Self {
			eye: Vec3::new(self.eye.x, self.eye.y - delta, self.eye.z),
			look_at: Vec3::new(self.look_at.x, y, self.look_at.z),
			..self
		}
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

/// View-space \(x / z\) range of the AABB corners (tan-space, not NDC).
fn projected_tan_x(
	aabb: Aabb3d,
	look_at: Vec3,
	forward: Vec3,
	right: Vec3,
	dist: f32,
) -> (f32, f32) {
	let eye = look_at - forward * dist;
	let mut min_x = f32::MAX;
	let mut max_x = f32::MIN;
	for corner in aabb_corners(aabb) {
		let rel = corner - eye;
		let z = rel.dot(forward).max(1e-4);
		let x = rel.dot(right) / z;
		min_x = min_x.min(x);
		max_x = max_x.max(x);
	}
	(min_x, max_x)
}

fn aabb_corners(aabb: Aabb3d) -> [Vec3; 8] {
	let (min, max) = (Vec3::from(aabb.min), Vec3::from(aabb.max));
	[
		Vec3::new(min.x, min.y, min.z),
		Vec3::new(min.x, min.y, max.z),
		Vec3::new(min.x, max.y, min.z),
		Vec3::new(min.x, max.y, max.z),
		Vec3::new(max.x, min.y, min.z),
		Vec3::new(max.x, min.y, max.z),
		Vec3::new(max.x, max.y, min.z),
		Vec3::new(max.x, max.y, max.z),
	]
}

fn perspective_fit_distance(
	aabb: Aabb3d,
	center: Vec3,
	forward: Vec3,
	right: Vec3,
	up: Vec3,
	fov: f32,
	aspect: f32,
	padding: f32,
	near: f32,
) -> f32 {
	let fill = (1.0 - padding.clamp(0.0, 0.9)).max(0.05);
	let half_y = (fov.max(1e-3) * 0.5).tan();
	let half_x = half_y * aspect.max(1e-3);
	let mut dist = near + 0.25;
	for corner in aabb_corners(aabb) {
		let offset = corner - center;
		let x = offset.dot(right).abs();
		let y = offset.dot(up).abs();
		let depth = offset.dot(forward);
		dist = dist
			.max(x / (half_x * fill) - depth)
			.max(y / (half_y * fill) - depth)
			.max(near + 0.05 - depth);
	}
	dist
}

fn orthographic_fit_scale(
	aabb: Aabb3d,
	center: Vec3,
	right: Vec3,
	up: Vec3,
	aspect: f32,
	padding: f32,
) -> f32 {
	let fill = (1.0 - padding.clamp(0.0, 0.9)).max(0.05);
	let mut half_x = 0.05_f32;
	let mut half_y = 0.05_f32;
	for corner in aabb_corners(aabb) {
		let offset = corner - center;
		half_x = half_x.max(offset.dot(right).abs());
		half_y = half_y.max(offset.dot(up).abs());
	}
	let needed_y = half_y / fill;
	let needed_x = half_x / (fill * aspect.max(1e-3));
	needed_x.max(needed_y) * 2.0
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

	#[test]
	fn fit_extents_centers_a_shifted_box() -> anyhow::Result<()> {
		let view = CaptureView::look_at(Vec3::new(0.0, 0.0, 8.0), Vec3::ZERO).perspective(0.8);
		let aabb = Aabb3d::from_min_max(Vec3::new(2.0, -0.5, -0.5), Vec3::new(3.0, 0.5, 0.5));
		let fitted = view.fit_extents(aabb, 1.0, 0.1);
		assert!((fitted.look_at.y).abs() < 1e-3);
		let forward = (fitted.look_at - fitted.eye).normalize();
		let right = forward.cross(fitted.up).normalize();
		let dist = fitted.eye.distance(fitted.look_at);
		let (min_x, max_x) = projected_tan_x(aabb, fitted.look_at, forward, right, dist);
		assert!(((min_x + max_x) * 0.5).abs() < 0.02);
		assert!(dist < 8.0);
		Ok(())
	}

	#[test]
	fn fit_extents_centers_projected_width_on_a_three_quarter_view() -> anyhow::Result<()> {
		let view = CaptureView::look_at(Vec3::new(2.0, 1.0, 2.0), Vec3::new(0.0, 0.2, 0.0))
			.perspective(0.75);
		let aabb = Aabb3d::from_min_max(Vec3::new(-0.15, -0.1, -1.6), Vec3::new(0.15, 0.25, 0.35));
		let fitted = view.fit_extents(aabb, 2.3, 0.05);
		assert!((fitted.look_at.y - 0.2).abs() < 1e-4);
		let forward = (fitted.look_at - fitted.eye).normalize();
		let right = forward.cross(fitted.up).normalize();
		let dist = fitted.eye.distance(fitted.look_at);
		let (min_x, max_x) = projected_tan_x(aabb, fitted.look_at, forward, right, dist);
		assert!(((min_x + max_x) * 0.5).abs() < 0.03);
		Ok(())
	}

	#[test]
	fn more_padding_pulls_the_camera_back() -> anyhow::Result<()> {
		let view = CaptureView::look_at(Vec3::new(2.0, 1.0, 2.0), Vec3::ZERO).perspective(0.75);
		let aabb = Aabb3d::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5));
		let tight = view.fit_extents(aabb, 1.0, 0.05);
		let loose = view.fit_extents(aabb, 1.0, 0.35);
		assert!(loose.eye.distance(loose.look_at) > tight.eye.distance(tight.look_at));
		Ok(())
	}

	#[test]
	fn fit_extents_keeps_authored_look_at_height() -> anyhow::Result<()> {
		let view = CaptureView::look_at(Vec3::new(2.0, 1.2, 2.0), Vec3::new(0.0, 0.2, 0.0));
		let aabb = Aabb3d::from_min_max(Vec3::new(-0.5, -1.0, -0.5), Vec3::new(0.5, 0.0, 0.5));
		let fitted = view.fit_extents(aabb, 1.0, 0.1);
		assert!((fitted.look_at.y - 0.2).abs() < 1e-4);
		Ok(())
	}
}
