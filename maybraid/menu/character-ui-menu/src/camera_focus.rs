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
}
