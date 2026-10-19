//! Representative stills: clip + progress, captured through [`CaptureSubject`].

use std::borrow::Cow;

use bevy::prelude::Vec3;

use crate::id::CaptureId;
use crate::subject::{CaptureScene, CaptureSubject};
use crate::view::CaptureView;

/// Isolated host kind for a posed character (or any app-realized still).
pub const HOST_REPRESENTATIVE_POSE: &str = "representative-pose";

/// Full-body three-quarter view. [`crate::CaptureFraming::FitExtents`] then
/// trucks and crops so the posed silhouette fills the icon.
pub fn representative_pose_view() -> CaptureView {
	CaptureView::look_at(Vec3::new(2.4, 1.35, 2.6), Vec3::new(0.0, 0.95, 0.0))
		.perspective(CaptureView::DEFAULT_FOV)
}

/// A subject whose capture scene is a posed host at a clip + progress.
///
/// Implementors describe the still (`clip_key` + `clip_progress`). A blanket
/// [`CaptureSubject`] impl turns that into an isolated
/// [`CaptureScene::Host`]. `pose_revision` should change when the live clip's
/// sampled pose at that progress changes, so disk-cache keys follow
/// choreography edits.
pub trait RepresentativePose: Send + Sync + Clone + 'static {
	/// Stable clip identity, including laterality (`power-strafe-left`).
	fn clip_key(&self) -> Cow<'static, str>;

	/// Normalized clip progress in `[0, 1]`.
	fn clip_progress(&self) -> f32;

	/// English / catalog slug for the PNG stem.
	fn pose_label(&self) -> Cow<'static, str> {
		self.clip_key()
	}

	/// Fingerprint of the posed appearance. Default `0` is progress-only.
	fn pose_revision(&self) -> u64 {
		0
	}
}

impl<T: RepresentativePose> CaptureSubject for T {
	fn capture_id(&self) -> CaptureId {
		CaptureId::new(pose_capture_label(
			self.clip_key().as_ref(),
			self.clip_progress(),
			self.pose_revision(),
		))
	}

	fn capture_scene(&self) -> CaptureScene {
		CaptureScene::host(
			HOST_REPRESENTATIVE_POSE,
			pose_capture_label(
				self.clip_key().as_ref(),
				self.clip_progress(),
				self.pose_revision(),
			),
		)
	}

	fn preferred_view(&self) -> CaptureView {
		representative_pose_view()
	}
}

pub fn pose_capture_label(clip_key: &str, progress: f32, revision: u64) -> String {
	format!("pose:{clip_key}:t={:x}:r={revision:x}", progress.to_bits())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Clone)]
	struct DemoPose {
		key: &'static str,
		progress: f32,
		revision: u64,
	}

	impl RepresentativePose for DemoPose {
		fn clip_key(&self) -> Cow<'static, str> {
			Cow::Borrowed(self.key)
		}

		fn clip_progress(&self) -> f32 {
			self.progress
		}

		fn pose_revision(&self) -> u64 {
			self.revision
		}
	}

	#[test]
	fn blanket_subject_uses_the_representative_host() -> anyhow::Result<()> {
		let pose = DemoPose { key: "power-dash", progress: 0.5, revision: 0xabc };
		assert_eq!(pose.capture_id().as_str(), pose_capture_label("power-dash", 0.5, 0xabc));
		assert!(matches!(
			pose.capture_scene(),
			CaptureScene::Host { kind, fragment }
				if kind == HOST_REPRESENTATIVE_POSE && fragment.contains("power-dash")
		));
		assert_eq!(pose.preferred_view(), representative_pose_view());
		Ok(())
	}

	#[test]
	fn revision_and_progress_change_the_id() {
		let a = DemoPose { key: "power-dash", progress: 0.5, revision: 1 };
		let b = DemoPose { key: "power-dash", progress: 0.5, revision: 2 };
		let c = DemoPose { key: "power-dash", progress: 0.42, revision: 1 };
		assert_ne!(a.capture_id(), b.capture_id());
		assert_ne!(a.capture_id(), c.capture_id());
	}
}
