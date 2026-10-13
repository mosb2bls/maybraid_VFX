use crate::CameraFocus;

#[derive(Clone, Debug, PartialEq)]
pub struct Section<T> {
	pub label: &'static str,
	pub value: T,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> Section<T> {
	pub const fn new(label: &'static str, value: T) -> Self {
		Self { label, value, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}
}
