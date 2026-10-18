//! Per-frame character articulation: clips, mailbox, terrain pitch.
//!
//! Recipes (`characters`) stamp host identity. This crate syncs host
//! motion markers from the shown LOD band (or visual High when there is no
//! [`lod::LodSceneHost`]), clamps mailbox work from plant
//! [`intelligence_lod::IntelligenceLod`] (missing = Near; visible High plants
//! keep mailbox writes even when thinking is Mid/Far), and realizes clips /
//! pitch. See [README.md](../README.md).
//!
//! This crate does **not** implement [`lod::LodScene`] or species recipes.

pub mod clip;
pub mod clip_cache;
pub mod elevation;
pub mod mailbox;
pub mod markers;
pub mod pitch;
pub mod plant;
pub mod plugin;
pub mod policy;
pub mod rig;
pub mod shown;
pub mod sync;

pub use clip::{
	AnimClip, AnimId, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams, TuckedFlipParams,
	TwoFootedTuckedFlipParams,
};
pub use clip_cache::{
	apply_evaluated_sample, clip_bone_mask, parameters_key, AnimClipCache, AnimClipCacheSettings,
	AnimClipCacheStats, CachedClipSample, ClipParametersId, ClipParametersKey, ClipVariantKey,
	EvaluatedBoneOutput, PreparedClip, RigVariantId, SamplingSettings, CURRENT_CLIP_REVISION,
	DEFAULT_MAX_UNBOUNDED_BINS, DEFAULT_MAX_VARIANTS,
};
pub use elevation::{
	apply_terrain_pitch, draw_terrain_pitch_probes, is_local_visual_child, probe_origin,
	DrawTerrainPitchProbes,
};
pub use mailbox::{
	apply_anim_mailbox, prepare_anim_mailbox, select_mailbox_applies, tick_anim_mailbox, AnimBone,
	AnimMailbox, AnimProgress, MailboxApplyLimits, MailboxApplySet,
};
pub use markers::{
	AnimateBones, AnimateEffects, ApplyTerrainPitch, SuspendAnimation, SuspendTerrainPitch,
};
pub use pitch::{CharacterHeading, TerrainPitch};
pub use plugin::{CharacterMotionPlugin, CharacterMotionSystems};
pub use policy::{clamp_intelligence, motion_policy, MotionPolicy};
pub use rig::{
	bone_map_ready, missing_landmark_bones, BoneMap, CharacterRig, CharacterRigRole,
	RigSkeletonKind,
};
pub use shown::shown_level_root;
pub use sync::sync_motion_markers;
