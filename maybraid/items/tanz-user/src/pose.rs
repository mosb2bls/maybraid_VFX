//! [`RepresentativePose`] for starter Tanzes: live clip + readable still.

use std::borrow::Cow;

use characters::fingerprint_humanoid_clip;
use screen_capture::RepresentativePose;

use crate::stack::TanzId;

impl RepresentativePose for TanzId {
	fn clip_key(&self) -> Cow<'static, str> {
		Cow::Owned(self.definition().clip.capture_key())
	}

	fn clip_progress(&self) -> f32 {
		self.definition().clip.representative_progress()
	}

	fn pose_label(&self) -> Cow<'static, str> {
		Cow::Borrowed(self.script_id())
	}

	fn pose_revision(&self) -> u64 {
		fingerprint_humanoid_clip(self.definition().clip, self.clip_progress())
	}
}

#[cfg(test)]
mod tests {
	use characters::{fingerprint_humanoid_clip, AnimClip};
	use screen_capture::{CaptureSubject, HOST_REPRESENTATIVE_POSE};

	use super::*;

	#[test]
	fn representative_progress_comes_from_the_live_clip() {
		assert_eq!(
			TanzId::ForwardDash.clip_progress(),
			AnimClip::power_dash().representative_progress()
		);
		assert_eq!(
			TanzId::LeftStrafe.clip_progress(),
			AnimClip::power_strafe_left().representative_progress()
		);
		assert_eq!(
			TanzId::RightStrafe.clip_progress(),
			AnimClip::power_strafe_right().representative_progress()
		);
		assert_eq!(
			TanzId::HealingSalute.clip_progress(),
			AnimClip::jab().representative_progress()
		);
		assert_eq!(TanzId::LeftStrafe.clip_key().as_ref(), "power-strafe-left");
		assert_eq!(TanzId::RightStrafe.clip_key().as_ref(), "power-strafe-right");
		assert_eq!(TanzId::ForwardDash.clip_key().as_ref(), "power-dash");
		assert_eq!(TanzId::HealingSalute.clip_key().as_ref(), "jab");
	}

	#[test]
	fn capture_id_follows_the_sampled_pose() {
		let dash = TanzId::ForwardDash;
		let left = TanzId::LeftStrafe;
		assert_eq!(
			dash.pose_revision(),
			fingerprint_humanoid_clip(dash.definition().clip, dash.clip_progress())
		);
		assert_ne!(dash.capture_id(), left.capture_id());
		assert!(matches!(
			dash.capture_scene(),
			screen_capture::CaptureScene::Host { kind, .. } if kind == HOST_REPRESENTATIVE_POSE
		));
		let rest = fingerprint_humanoid_clip(AnimClip::power_dash(), 0.0);
		assert_ne!(
			dash.pose_revision(),
			rest,
			"held dash pose must be read from the live clip, not rest"
		);
	}

	#[test]
	fn mirrored_strafes_have_distinct_revisions() {
		assert_ne!(TanzId::LeftStrafe.pose_revision(), TanzId::RightStrafe.pose_revision());
	}
}
