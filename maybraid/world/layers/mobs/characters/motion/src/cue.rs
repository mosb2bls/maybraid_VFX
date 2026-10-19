//! Normalized timing cues for gameplay (VFX, audio, rumble). Declared per clip, not sampled from pose.
//!
//! [`tick_anim_clip_cues`] mirrors mailbox progress and exposes edge crossings on
//! [`AnimClipCues`] so consumers never duplicate phase math inside samplers.

use bevy::prelude::*;

use crate::clip::{AnimClip, AnimId, AnimRefRoot};
use crate::mailbox::{AnimBindingGap, AnimMailbox};
use crate::markers::SuspendAnimation;
use crate::rig::{CharacterRig, CharacterRigRole};
use character_animations::animations::{
	SLEEP_ENTER_IMPACT_NORM, SONIC_EMIT_NORM, WAKE_IMPACT_NORM,
};
use character_animations::{ClipTimePolicy, Progress};

/// Gameplay-facing cue kinds for Epiphant combat / rest clips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimCueKind {
	SonicEmit,
	GroundImpact,
}

/// One-shot marker in normalized clip time `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimCueMarker {
	pub at: f32,
}

impl AnimCueMarker {
	pub const fn new(at: f32) -> Self {
		Self { at }
	}

	pub fn crossed(prev: f32, now: f32, marker: Self) -> bool {
		prev < marker.at && now >= marker.at
	}
}

/// Static cue sheet for a clip variant. Identity clips return an empty sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AnimCueSheet {
	pub sonic_emit: Option<AnimCueMarker>,
	pub ground_impact: Option<AnimCueMarker>,
}

impl AnimCueSheet {
	pub fn marker(self, kind: AnimCueKind) -> Option<AnimCueMarker> {
		match kind {
			AnimCueKind::SonicEmit => self.sonic_emit,
			AnimCueKind::GroundImpact => self.ground_impact,
		}
	}

	pub fn crossed(self, kind: AnimCueKind, prev: f32, now: f32) -> bool {
		self.marker(kind)
			.is_some_and(|marker| AnimCueMarker::crossed(prev, now, marker))
	}
}

/// Cue sheet for a clip identity. Knob changes on the same [`AnimId`] keep the same sheet.
pub fn cue_sheet(clip: AnimClip) -> AnimCueSheet {
	match clip {
		AnimClip::EpiphantBray(_) => AnimCueSheet {
			sonic_emit: Some(AnimCueMarker::new(SONIC_EMIT_NORM)),
			ground_impact: None,
		},
		AnimClip::EpiphantSleepEnter(_) => AnimCueSheet {
			sonic_emit: None,
			ground_impact: Some(AnimCueMarker::new(SLEEP_ENTER_IMPACT_NORM)),
		},
		AnimClip::EpiphantWake(_) => AnimCueSheet {
			sonic_emit: None,
			ground_impact: Some(AnimCueMarker::new(WAKE_IMPACT_NORM)),
		},
		_ => AnimCueSheet::default(),
	}
}

/// Published each frame on body hosts with an [`AnimMailbox`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimClipCues {
	pub clip: AnimId,
	/// Mailbox clip seconds before policy remap (used to detect restarts).
	pub clip_time_secs: f32,
	pub progress: f32,
	pub prev_progress: f32,
	pub sheet: AnimCueSheet,
}

impl AnimClipCues {
	pub fn crossed(&self, kind: AnimCueKind) -> bool {
		self.sheet.crossed(kind, self.prev_progress, self.progress)
	}

	pub fn sonic_emit_crossed(&self) -> bool {
		self.crossed(AnimCueKind::SonicEmit)
	}

	pub fn ground_impact_crossed(&self) -> bool {
		self.crossed(AnimCueKind::GroundImpact)
	}
}

impl Default for AnimClipCues {
	fn default() -> Self {
		Self {
			clip: AnimId::Still,
			clip_time_secs: 0.0,
			progress: 0.0,
			prev_progress: 0.0,
			sheet: AnimCueSheet::default(),
		}
	}
}

/// Advance cue progress and reset crossing history on clip changes / restarts.
pub fn advance_anim_clip_cues(cues: &mut AnimClipCues, clip: AnimClip, clip_time_secs: f32) {
	let clip_id = clip.id();
	let progress = normalized_progress(clip, clip_time_secs);
	let segment_start = cues.clip != clip_id
		|| clip_time_secs + f32::EPSILON < cues.clip_time_secs
		|| progress + 0.5 < cues.progress;
	cues.prev_progress = if segment_start { 0.0 } else { cues.progress };
	cues.progress = progress;
	cues.clip = clip_id;
	cues.clip_time_secs = clip_time_secs;
	cues.sheet = cue_sheet(clip);
}

/// Insert [`AnimClipCues`] on body mailboxes once the mailbox exists.
#[allow(clippy::type_complexity)]
pub fn prepare_anim_clip_cues(
	mut commands: Commands,
	hosts: Query<
		(Entity, &CharacterRig, Has<AnimClipCues>),
		(With<AnimMailbox>, Without<AnimBindingGap>),
	>,
) {
	for (entity, rig, has) in &hosts {
		if rig.role != CharacterRigRole::Body || has {
			continue;
		}
		commands.entity(entity).insert(AnimClipCues::default());
	}
}

/// Advance cue state in lockstep with mailbox clip progress (run after [`tick_anim_mailbox`]).
#[allow(clippy::type_complexity)]
pub fn tick_anim_clip_cues(
	mut hosts: Query<
		(&AnimRefRoot, &AnimMailbox, &mut AnimClipCues),
		(With<AnimMailbox>, Without<SuspendAnimation>),
	>,
) {
	for (root, mailbox, mut cues) in &mut hosts {
		advance_anim_clip_cues(&mut *cues, root.0.clip, mailbox.clip_time_secs());
	}
}

fn normalized_progress(clip: AnimClip, clip_time: f32) -> f32 {
	match clip.time_policy() {
		ClipTimePolicy::Clamp { .. } => Progress(clip_time).clamp(),
		ClipTimePolicy::Cycle { .. } => Progress(clip_time).cycle(),
		ClipTimePolicy::Unbounded => clip_time,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::animations::EpiphantBray;

	#[test]
	fn bray_sheet_declares_sonic_emit_only() {
		let sheet = cue_sheet(AnimClip::epiphant_bray());
		assert!(sheet.sonic_emit.is_some());
		assert!(sheet.ground_impact.is_none());
		assert_eq!(sheet.sonic_emit.unwrap().at, SONIC_EMIT_NORM);
	}

	#[test]
	fn sleep_and_wake_impact_markers_are_stable() {
		let enter = cue_sheet(AnimClip::epiphant_sleep_enter());
		let wake = cue_sheet(AnimClip::epiphant_wake());
		assert_eq!(enter.ground_impact.unwrap().at, SLEEP_ENTER_IMPACT_NORM);
		assert_eq!(wake.ground_impact.unwrap().at, WAKE_IMPACT_NORM);
	}

	#[test]
	fn crossing_detects_forward_passage_through_marker() {
		let marker = AnimCueMarker::new(0.5);
		assert!(!AnimCueMarker::crossed(0.4, 0.49, marker));
		assert!(AnimCueMarker::crossed(0.49, 0.51, marker));
	}

	#[test]
	fn epiphant_bray_knobs_do_not_change_cue_sheet() {
		let a = cue_sheet(AnimClip::EpiphantBray(EpiphantBray::default()));
		let b = cue_sheet(AnimClip::EpiphantBray(EpiphantBray {
			flick_turn: 0.9,
			flick_nod: 0.1,
			anticipation: 0.3,
		}));
		assert_eq!(a, b);
	}

	#[test]
	fn clip_change_allows_first_frame_sonic_crossing() {
		let bray = AnimClip::epiphant_bray();
		let mut cues = AnimClipCues::default();
		advance_anim_clip_cues(&mut cues, AnimClip::quadruped_run(), 0.95);
		assert!(!cues.sonic_emit_crossed());

		advance_anim_clip_cues(&mut cues, bray, 0.34);
		assert!(
			cues.sonic_emit_crossed(),
			"first bray sample past SONIC_EMIT_NORM should fire after clip change"
		);
	}

	#[test]
	fn restart_allows_first_frame_sonic_crossing() {
		let bray = AnimClip::epiphant_bray();
		let mut cues = AnimClipCues::default();
		advance_anim_clip_cues(&mut cues, bray, 0.25);
		advance_anim_clip_cues(&mut cues, bray, 0.40);
		assert!(cues.sonic_emit_crossed());
		advance_anim_clip_cues(&mut cues, bray, 0.90);
		assert!(!cues.sonic_emit_crossed());

		advance_anim_clip_cues(&mut cues, bray, 0.34);
		assert!(
			cues.sonic_emit_crossed(),
			"restarted bray sample past SONIC_EMIT_NORM should fire"
		);
	}
}
