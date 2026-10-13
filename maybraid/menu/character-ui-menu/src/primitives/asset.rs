use bevy_math::Vec3;

use crate::CameraFocus;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbnailCamera {
	pub position: Vec3,
	pub look_at: Vec3,
}

impl ThumbnailCamera {
	pub const DEFAULT: Self = Self { position: Vec3::new(0.0, 0.45, 1.55), look_at: Vec3::ZERO };

	pub const fn new(position: Vec3, look_at: Vec3) -> Self {
		Self { position, look_at }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdentifiedAsset {
	pub id: &'static str,
	pub label: &'static str,
	pub path: &'static str,
	pub thumbnail_camera: ThumbnailCamera,
}

impl IdentifiedAsset {
	pub const fn new(id: &'static str, label: &'static str, path: &'static str) -> Self {
		Self { id, label, path, thumbnail_camera: ThumbnailCamera::DEFAULT }
	}

	pub const fn with_thumbnail_camera(mut self, thumbnail_camera: ThumbnailCamera) -> Self {
		self.thumbnail_camera = thumbnail_camera;
		self
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetSingleSelect<T> {
	pub value: T,
	pub camera_focus: Option<CameraFocus>,
}

impl<T> AssetSingleSelect<T> {
	pub const fn new(value: T) -> Self {
		Self { value, camera_focus: None }
	}

	pub const fn with_camera_focus(mut self, camera_focus: CameraFocus) -> Self {
		self.camera_focus = Some(camera_focus);
		self
	}
}
