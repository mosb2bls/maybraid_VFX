//! [`RepresentativePose`] for starter Tanzes: live clip + readable still.

use std::borrow::Cow;

use character_items::{icon_catalog_key_for_label, Inventory, InventoryItem, TanzId};
use characters::fingerprint_humanoid_clip;
use item_icons::{request_pose_icon, CaptureClient, ItemIconDiscovery, PoseCaptureSubjects};
use screen_capture::{CaptureSubject, RepresentativePose};

use crate::TanzDefinitionExt;

/// Local capture adapter. [`TanzId`] and [`RepresentativePose`] are both
/// foreign to this crate once identity lives in `character-items`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TanzPose(pub TanzId);

impl TanzPose {
	pub fn icon_catalog_key(self) -> u64 {
		icon_catalog_key_for_label(self.capture_id().as_str())
	}
}

/// Pose-capture every owned Tanz, equipped or not.
pub fn request_inventory_tanz_icons(
	inventory: &Inventory,
	discovery: &mut ItemIconDiscovery,
	poses: &mut PoseCaptureSubjects,
	client: &mut CaptureClient,
) {
	for id in inventory.items.iter().filter_map(InventoryItem::tanz_id) {
		let pose = TanzPose(id);
		request_pose_icon(
			&pose,
			id.definition().clip,
			pose.clip_progress(),
			discovery,
			poses,
			client,
		);
	}
}

impl RepresentativePose for TanzPose {
	fn clip_key(&self) -> Cow<'static, str> {
		Cow::Owned(self.0.definition().clip.capture_key())
	}

	fn clip_progress(&self) -> f32 {
		self.0.definition().clip.representative_progress()
	}

	fn pose_label(&self) -> Cow<'static, str> {
		Cow::Borrowed(self.0.script_id())
	}

	fn pose_revision(&self) -> u64 {
		fingerprint_humanoid_clip(self.0.definition().clip, self.clip_progress())
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
			TanzPose(TanzId::ForwardDash).clip_progress(),
			AnimClip::power_dash().representative_progress()
		);
		assert_eq!(
			TanzPose(TanzId::LeftStrafe).clip_progress(),
			AnimClip::power_strafe_left().representative_progress()
		);
		assert_eq!(
			TanzPose(TanzId::RightStrafe).clip_progress(),
			AnimClip::power_strafe_right().representative_progress()
		);
		assert_eq!(
			TanzPose(TanzId::HealingSalute).clip_progress(),
			AnimClip::jab().representative_progress()
		);
		assert_eq!(TanzPose(TanzId::LeftStrafe).clip_key().as_ref(), "power-strafe-left");
		assert_eq!(TanzPose(TanzId::RightStrafe).clip_key().as_ref(), "power-strafe-right");
		assert_eq!(TanzPose(TanzId::ForwardDash).clip_key().as_ref(), "power-dash");
		assert_eq!(TanzPose(TanzId::HealingSalute).clip_key().as_ref(), "jab");
	}

	#[test]
	fn capture_id_follows_the_sampled_pose() {
		let dash = TanzPose(TanzId::ForwardDash);
		let left = TanzPose(TanzId::LeftStrafe);
		assert_eq!(
			dash.pose_revision(),
			fingerprint_humanoid_clip(dash.0.definition().clip, dash.clip_progress())
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
		assert_ne!(
			TanzPose(TanzId::LeftStrafe).pose_revision(),
			TanzPose(TanzId::RightStrafe).pose_revision()
		);
	}
}
