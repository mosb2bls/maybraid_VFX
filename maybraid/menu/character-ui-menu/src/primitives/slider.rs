use crate::CameraFocus;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider {
	pub value: f32,
	pub min: f32,
	pub max: f32,
	pub step: f32,
	pub camera_focus: Option<CameraFocus>,
}

impl Slider {
	pub const fn new(value: f32, min: f32, max: f32, step: f32) -> Self {
		Self { value, min, max, step, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}

	pub fn apply_delta(mut self, delta: f32) -> Self {
		self.value = (self.value + delta).clamp(self.min, self.max);
		self
	}
}
