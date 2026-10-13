use crate::CameraFocus;

#[derive(Clone, Debug, PartialEq)]
pub struct MultiSelect<T> {
	pub selected: Vec<T>,
	pub camera_focus: Option<CameraFocus>,
}

impl<T: Copy + PartialEq> MultiSelect<T> {
	pub fn new(selected: Vec<T>) -> Self {
		Self { selected, camera_focus: None }
	}

	pub fn toggle(&mut self, item: T) {
		if let Some(index) = self.selected.iter().position(|value| *value == item) {
			self.selected.remove(index);
		} else {
			self.selected.push(item);
		}
	}

	pub fn contains(&self, item: T) -> bool {
		self.selected.iter().any(|value| *value == item)
	}
}
