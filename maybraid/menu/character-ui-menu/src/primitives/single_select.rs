use crate::CameraFocus;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleSelect<T> {
	pub value: T,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> SingleSelect<T> {
	pub const fn new(value: T) -> Self {
		Self { value, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}
}
