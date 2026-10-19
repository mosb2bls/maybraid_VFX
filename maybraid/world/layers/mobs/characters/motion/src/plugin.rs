//! Mailbox + host marker sync. Elevation is registered by the app with a probe.

use bevy::prelude::*;

use intelligence_lod::{IntelligenceFocus, IntelligenceLookFrame, IntelligencePriority};

use crate::clip_cache::AnimClipCache;
use crate::cue::{prepare_anim_clip_cues, tick_anim_clip_cues};
use crate::elevation::{DrawTerrainPitchProbes, draw_terrain_pitch_probes};
use crate::mailbox::{
	MailboxApplyLimits, MailboxApplySet, apply_anim_mailbox, init_working_pose,
	prepare_anim_mailbox, publish_anim_mailbox, select_mailbox_applies, tick_anim_mailbox,
};
use crate::sync::sync_motion_markers;

/// Per-frame articulation sets. Recipes schedule structural pose **before**
/// [`Self::Anim`]. Playgrounds schedule elevation after physics / locomotion.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterMotionSystems {
	/// Sync host markers from the shown LOD band, then prepare + tick + apply clips.
	Anim,
	/// `apply_terrain_pitch::<P>` — add this system yourself with a probe.
	/// Sample from the capsule parent when the visual is a local child;
	/// exclude / suspend via ancestors.
	Elevation,
}

/// Explicit pose pipeline after clip tick.
///
/// Target order is `SampleBase` → `InitWorking` → `Overlay` → `FinalCorrections`
/// → `Publish`. This slice keeps [`Self::Publish`] immediately after
/// [`Self::InitWorking`] (still inside [`CharacterMotionSystems::Anim`]) so
/// firearm/grenade `TransformHelper` reads see this-frame base bones. Overlays
/// still patch their affected bones afterward. A follow-up moves `Publish`
/// after `FinalCorrections` once overlays stop writing scene transforms.
///
/// Item crates contribute typed overlay systems (no callback registry).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterPoseSystems {
	/// Sample/crossfade the immutable base pose and armature offset.
	SampleBase,
	/// Copy [`crate::AnimMailbox::base_pose`] into the shared working pose once.
	InitWorking,
	/// Item/action overlays mutate the working pose (typed components + masks).
	Overlay,
	/// Post-overlay corrections before a later unified publish.
	FinalCorrections,
	/// Write bone `Transform`s from the working pose (currently still the base).
	Publish,
}

/// Overlay precedence inside [`CharacterPoseSystems::Overlay`].
///
/// Grenade runs after firearm so a throw arm wins on overlapping right-arm
/// bones. Production still removes the firearm hold marker while a grenade
/// overlay is active; these sets are the schedule contract for tests and for
/// composing both overlays on the working pose.
///
/// Reuses [`character_rigs::authoring::humanoid_write_mask`] bone names (same
/// family as clip write masks and jab-guard arm isolation). Do not add a second
/// mask system.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterPoseOverlay {
	Firearm,
	Grenade,
}

impl CharacterPoseSystems {
	/// Named pose-pipeline sets. Tests can order dummy systems without running
	/// mailbox queries.
	pub fn configure(app: &mut App) {
		app.configure_sets(
			Update,
			(
				Self::SampleBase.in_set(CharacterMotionSystems::Anim),
				Self::InitWorking.after(Self::SampleBase).in_set(CharacterMotionSystems::Anim),
				Self::Publish.after(Self::InitWorking).in_set(CharacterMotionSystems::Anim),
				Self::Overlay.after(Self::Publish),
				Self::FinalCorrections.after(Self::Overlay),
				CharacterPoseOverlay::Firearm.in_set(Self::Overlay),
				CharacterPoseOverlay::Grenade
					.in_set(Self::Overlay)
					.after(CharacterPoseOverlay::Firearm),
			),
		);
	}
}

/// Clip mailbox and host motion-marker sync. Does not register elevation.
///
/// Apps that also run `characters` should order
/// [`CharacterMotionSystems::Anim`] after `CharacterHostSystems::Pose`.
pub struct CharacterMotionPlugin;

impl Plugin for CharacterMotionPlugin {
	fn build(&self, app: &mut App) {
		CharacterPoseSystems::configure(app);
		app.init_resource::<DrawTerrainPitchProbes>()
			.init_resource::<IntelligencePriority>()
			.init_resource::<IntelligenceLookFrame>()
			.init_resource::<IntelligenceFocus>()
			.init_resource::<MailboxApplyLimits>()
			.init_resource::<MailboxApplySet>()
			.init_resource::<AnimClipCache>()
			.configure_sets(
				Update,
				CharacterMotionSystems::Elevation.after(CharacterMotionSystems::Anim),
			)
			.add_systems(
				Update,
				(
					sync_motion_markers,
					prepare_anim_mailbox.after(sync_motion_markers),
					prepare_anim_clip_cues.after(prepare_anim_mailbox),
					select_mailbox_applies.after(prepare_anim_clip_cues),
					tick_anim_mailbox.after(select_mailbox_applies),
					tick_anim_clip_cues.after(tick_anim_mailbox),
					apply_anim_mailbox
						.after(tick_anim_clip_cues)
						.in_set(CharacterPoseSystems::SampleBase),
					init_working_pose.in_set(CharacterPoseSystems::InitWorking),
					publish_anim_mailbox.in_set(CharacterPoseSystems::Publish),
				)
					.in_set(CharacterMotionSystems::Anim),
			)
			.add_systems(PostUpdate, draw_terrain_pitch_probes);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Resource, Default)]
	struct PoseOrder(Vec<&'static str>);

	fn note_sample(mut order: ResMut<PoseOrder>) {
		order.0.push("sample");
	}

	fn note_init(mut order: ResMut<PoseOrder>) {
		order.0.push("init");
	}

	fn note_publish(mut order: ResMut<PoseOrder>) {
		order.0.push("publish");
	}

	fn note_firearm(mut order: ResMut<PoseOrder>) {
		order.0.push("firearm");
	}

	fn note_grenade(mut order: ResMut<PoseOrder>) {
		order.0.push("grenade");
	}

	fn note_final(mut order: ResMut<PoseOrder>) {
		order.0.push("final");
	}

	#[test]
	fn pose_pipeline_order_is_sample_init_publish_then_overlays() {
		let mut app = App::new();
		app.init_resource::<PoseOrder>();
		CharacterPoseSystems::configure(&mut app);
		app.add_systems(
			Update,
			(
				note_sample.in_set(CharacterPoseSystems::SampleBase),
				note_init.in_set(CharacterPoseSystems::InitWorking),
				note_publish.in_set(CharacterPoseSystems::Publish),
				note_firearm.in_set(CharacterPoseOverlay::Firearm),
				note_grenade.in_set(CharacterPoseOverlay::Grenade),
				note_final.in_set(CharacterPoseSystems::FinalCorrections),
			),
		);
		app.update();
		assert_eq!(
			app.world().resource::<PoseOrder>().0,
			["sample", "init", "publish", "firearm", "grenade", "final"]
		);
	}
}
