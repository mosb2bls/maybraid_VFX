//! Capture request, resolution, and revision.

use crate::id::CaptureId;
use crate::lighting::CaptureLighting;
use crate::mode::CaptureMode;
use crate::subject::{CaptureScene, CaptureSubject};
use crate::view::{CaptureFraming, CaptureView};

/// Pixel size of the captured image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CaptureResolution {
	pub width: u32,
	pub height: u32,
}

impl CaptureResolution {
	pub const ICON: Self = Self { width: 256, height: 256 };

	pub const fn new(width: u32, height: u32) -> Self {
		Self { width, height }
	}

	pub const fn is_valid(self) -> bool {
		self.width > 0 && self.height > 0 && self.width <= 4096 && self.height <= 4096
	}

	pub fn cache_fragment(self) -> String {
		format!("{}x{}", self.width, self.height)
	}
}

/// Explicit invalidation token after asset or shader changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CaptureRevision(pub u32);

impl CaptureRevision {
	/// Bump when captured shaders, framing, or silhouette rules change.
	pub const CURRENT: Self = Self(25);

	// `Default` is 0 so an unset discovery resource regenerates on first sight.

	pub const fn new(revision: u32) -> Self {
		Self(revision)
	}
}

/// One capture job: subject appearance plus render settings.
#[derive(Clone, Debug, PartialEq)]
pub struct CaptureRequest {
	pub id: CaptureId,
	pub scene: CaptureScene,
	pub view: CaptureView,
	pub mode: CaptureMode,
	pub resolution: CaptureResolution,
	pub lighting: CaptureLighting,
	pub revision: CaptureRevision,
	/// Optional English slug for the PNG name. Not part of the cache hash.
	pub file_stem: String,
	pub framing: CaptureFraming,
}

impl CaptureRequest {
	pub fn from_subject(
		subject: &impl CaptureSubject,
		mode: CaptureMode,
		resolution: CaptureResolution,
		lighting: CaptureLighting,
		revision: CaptureRevision,
	) -> Self {
		Self {
			id: subject.capture_id(),
			scene: subject.capture_scene(),
			view: subject.preferred_view(),
			mode,
			resolution,
			lighting,
			revision,
			file_stem: String::new(),
			framing: CaptureFraming::Authored,
		}
	}

	/// Read back an already-painted GPU image. View, lighting, and framing
	/// are unused; the cache key still includes their defaults so prune
	/// can rebuild the same name without the live handle.
	pub fn from_image_handle(
		id: CaptureId,
		image: bevy::prelude::Handle<bevy::prelude::Image>,
		mode: CaptureMode,
		resolution: CaptureResolution,
		revision: CaptureRevision,
	) -> Self {
		Self {
			id,
			scene: CaptureScene::image(image),
			view: CaptureView::unused(),
			mode,
			resolution,
			lighting: CaptureLighting::studio(),
			revision,
			file_stem: String::new(),
			framing: CaptureFraming::Authored,
		}
	}

	pub fn with_file_stem(mut self, name: impl Into<String>) -> Self {
		self.file_stem = crate::id::sanitize_file_stem(&name.into());
		self
	}

	pub fn with_framing(mut self, framing: CaptureFraming) -> Self {
		self.framing = framing;
		self
	}

	pub fn validate(&self) -> Result<(), String> {
		if self.id.as_str().is_empty() {
			return Err(String::from("empty capture id"));
		}
		if !self.resolution.is_valid() {
			return Err(String::from("resolution must be in 1..=4096"));
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::subject::MeshCaptureSubject;

	#[test]
	fn from_subject_copies_id_and_mode() -> anyhow::Result<()> {
		let subject = MeshCaptureSubject::cuboid([0.2, 0.4, 0.8, 1.0]);
		let request = CaptureRequest::from_subject(
			&subject,
			CaptureMode::Silhouette,
			CaptureResolution::ICON,
			CaptureLighting::studio(),
			CaptureRevision::CURRENT,
		);
		assert_eq!(request.id, subject.capture_id());
		assert_eq!(request.mode, CaptureMode::Silhouette);
		assert_eq!(request.resolution, CaptureResolution::ICON);
		Ok(())
	}

	#[test]
	fn from_image_handle_uses_authored_defaults() {
		use bevy::prelude::Handle;

		let request = CaptureRequest::from_image_handle(
			CaptureId::new("skill-map:fireball:1"),
			Handle::default(),
			CaptureMode::Shaded,
			CaptureResolution::ICON,
			CaptureRevision::CURRENT,
		);
		assert!(matches!(request.scene, CaptureScene::Image(_)));
		assert_eq!(request.view, CaptureView::unused());
		assert_eq!(request.framing, CaptureFraming::Authored);
		assert_eq!(request.scene.cache_fragment(), "image");
	}
}
