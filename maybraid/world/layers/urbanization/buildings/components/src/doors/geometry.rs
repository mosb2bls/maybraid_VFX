//! Continuous door kit geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorGeometry {
	Frame15(DoorFrame15),
	Leaf(DoorLeaf),
}

impl Default for DoorGeometry {
	fn default() -> Self {
		Self::leaf()
	}
}

impl DoorGeometry {
	pub fn frame_15() -> Self {
		Self::Frame15(DoorFrame15)
	}

	pub fn leaf() -> Self {
		Self::Leaf(DoorLeaf)
	}
}

/// Alias kept for migration; prefer [`DoorGeometry`].
pub type Door = DoorGeometry;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoorFrame15;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DoorLeaf;
