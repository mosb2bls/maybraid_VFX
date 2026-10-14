use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

/// Rig hierarchy used for camera framing metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FocusRig {
	Body,
	Head,
}

/// Camera framing relative to a named rig socket bone.
///
/// Offsets are in world meters along the socket bone's local axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFocus {
	pub rig: FocusRig,
	pub socket: &'static str,
	pub camera_offset: Vec3,
	pub look_at_offset: Vec3,
}

impl CameraFocus {
	/// Menu / playground preview vertical FOV (45°).
	pub const BODY_PREVIEW_FOV: f32 = std::f32::consts::FRAC_PI_4;
	/// Margin around the bounding sphere so feet and crown stay inside the frame.
	pub const BODY_PREVIEW_PADDING: f32 = 1.35;

	pub const fn new(
		rig: FocusRig,
		socket: &'static str,
		camera_offset: Vec3,
		look_at_offset: Vec3,
	) -> Self {
		Self { rig, socket, camera_offset, look_at_offset }
	}

	/// Same framing for a creature shrunk (or grown) by `factor` overall.
	pub const fn scaled(self, factor: f32) -> Self {
		let camera = self.camera_offset;
		let look = self.look_at_offset;
		Self {
			camera_offset: Vec3::new(camera.x * factor, camera.y * factor, camera.z * factor),
			look_at_offset: Vec3::new(look.x * factor, look.y * factor, look.z * factor),
			..self
		}
	}

	/// Whole-body 3/4 framing that fits `bounds` in `fov` with a consistent margin.
	pub fn fit_preview_bounds(bounds: Aabb3d, fov: f32, view: Vec3) -> Self {
		let look_at = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		let mut radius = 0.0_f32;
		for x in [bounds.min.x, bounds.max.x] {
			for y in [bounds.min.y, bounds.max.y] {
				for z in [bounds.min.z, bounds.max.z] {
					radius = radius.max(look_at.distance(Vec3::new(x, y, z)));
				}
			}
		}
		let dist = Self::BODY_PREVIEW_PADDING * radius.max(0.05) / (fov.max(1e-3) * 0.5).tan();
		let dir = if view.length_squared() > 1e-8 { view.normalize() } else { Vec3::Z };
		Self::new(FocusRig::Body, "root", look_at + dir * dist, look_at)
	}

	/// True when every AABB corner sits inside the perspective frustum.
	pub fn contains_preview_bounds(self, bounds: Aabb3d, fov: f32, aspect: f32) -> bool {
		let eye = self.camera_offset;
		let look = self.look_at_offset;
		let forward = look - eye;
		if forward.length_squared() < 1e-8 {
			return false;
		}
		let forward = forward.normalize();
		let mut right = forward.cross(Vec3::Y);
		if right.length_squared() < 1e-8 {
			right = forward.cross(Vec3::X);
		}
		let right = right.normalize();
		let up = right.cross(forward).normalize();
		let half_v = (fov.max(1e-3) * 0.5).tan();
		let half_h = half_v * aspect.max(1e-3);
		for x in [bounds.min.x, bounds.max.x] {
			for y in [bounds.min.y, bounds.max.y] {
				for z in [bounds.min.z, bounds.max.z] {
					let rel = Vec3::new(x, y, z) - eye;
					let depth = rel.dot(forward);
					if depth <= 1e-4 {
						return false;
					}
					if (rel.dot(right) / depth).abs() > half_h + 1e-4 {
						return false;
					}
					if (rel.dot(up) / depth).abs() > half_v + 1e-4 {
						return false;
					}
				}
			}
		}
		true
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fit_preview_bounds_contains_a_shifted_box() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-0.4, -0.9, -0.4), Vec3::new(0.4, 0.9, 0.4));
		let focus = CameraFocus::fit_preview_bounds(bounds, CameraFocus::BODY_PREVIEW_FOV, Vec3::Z);
		assert!(focus.contains_preview_bounds(bounds, CameraFocus::BODY_PREVIEW_FOV, 1.0));
		assert!(focus.look_at_offset.abs_diff_eq(Vec3::ZERO, 1e-4));
		assert!(focus.camera_offset.z > 0.0);
	}

	#[test]
	fn taller_bounds_push_the_camera_back() {
		let short = Aabb3d::from_min_max(Vec3::new(-0.2, -0.2, -0.2), Vec3::new(0.2, 0.2, 0.2));
		let tall = Aabb3d::from_min_max(Vec3::new(-0.4, -1.5, -0.4), Vec3::new(0.4, 1.5, 0.4));
		let short_focus =
			CameraFocus::fit_preview_bounds(short, CameraFocus::BODY_PREVIEW_FOV, Vec3::Z);
		let tall_focus =
			CameraFocus::fit_preview_bounds(tall, CameraFocus::BODY_PREVIEW_FOV, Vec3::Z);
		assert!(
			tall_focus.camera_offset.distance(tall_focus.look_at_offset)
				> short_focus.camera_offset.distance(short_focus.look_at_offset)
		);
	}
}
