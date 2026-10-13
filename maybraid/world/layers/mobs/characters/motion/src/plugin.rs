//! Mailbox + host marker sync. Elevation is registered by the app with a probe.

use bevy::prelude::*;

use intelligence_lod::{IntelligenceFocus, IntelligenceLookFrame, IntelligencePriority};

use crate::clip_cache::AnimClipCache;
use crate::elevation::{draw_terrain_pitch_probes, DrawTerrainPitchProbes};
use crate::mailbox::{
	apply_anim_mailbox, prepare_anim_mailbox, select_mailbox_applies, tick_anim_mailbox,
	MailboxApplyLimits, MailboxApplySet,
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

/// Clip mailbox and host motion-marker sync. Does not register elevation.
///
/// Apps that also run `characters` should order
/// [`CharacterMotionSystems::Anim`] after `CharacterHostSystems::Pose`.
pub struct CharacterMotionPlugin;

impl Plugin for CharacterMotionPlugin {
	fn build(&self, app: &mut App) {
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
					select_mailbox_applies.after(prepare_anim_mailbox),
					tick_anim_mailbox.after(select_mailbox_applies),
					apply_anim_mailbox.after(tick_anim_mailbox),
				)
					.in_set(CharacterMotionSystems::Anim),
			)
			.add_systems(PostUpdate, draw_terrain_pitch_probes);
	}
}
