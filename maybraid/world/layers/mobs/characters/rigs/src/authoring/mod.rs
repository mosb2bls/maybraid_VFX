//! V0 pose authoring and indexed pose buffers.
//!
//! Clip evaluation is a compatibility mapping onto the imported parent-space
//! swing / flex / twist compose. Semantic field names are the authoring API;
//! they do not by themselves mean character-space anatomical hinges. See
//! [`super::articulation::compose_parent_rotation`] and the per-family
//! `bone_axis` tables.
//!
//! Character space is the armature local frame: **+X** right, **+Y** up, **+Z**
//! fight-forward. Bevy world space stays **+Y** up and **−Z** camera-forward.

mod binding;
mod buffer;
mod forelimbed;
mod frame;
mod humanoid;
mod quadruped;
mod space;

pub use binding::{BoneId, RigBinding, RigDefinition, RigMetrics, SkeletonFamily};
pub use buffer::{ArmatureOffset, BlendCurve, PoseBuffer, PoseScratch};
pub use forelimbed::{
	forelimbed_v0_definition, resolve_forelimbed, ForelimbedPose, FORELIMBED_V0_BONES,
	FORELIMBED_V0_COUNT,
};
pub use frame::{JointAngles, JointFrame};
pub use humanoid::{
	apply_humanoid_glb_rest, bone_axis as humanoid_bone_axis, humanoid_bone_bit,
	humanoid_parent_rotation_delta, humanoid_pose_write_mask, humanoid_v0_definition,
	humanoid_write_mask, identity_binding as humanoid_identity_binding, resolve_humanoid,
	resolve_humanoid_masked, ArmAim, ArmPose, HumanoidPose, LegPose, NeckBone, NeckPose, SpinePose,
	HUMANOID_FEMUR_AXIS, HUMANOID_GLB_FEMUR, HUMANOID_GLB_PELVIS_L, HUMANOID_GLB_PELVIS_R,
	HUMANOID_GLB_SHOULDER_L, HUMANOID_GLB_SHOULDER_R, HUMANOID_RIGHT_FEMUR_AXIS,
	HUMANOID_RIGHT_FLEX_AXIS, HUMANOID_RIGHT_SHIN_AXIS, HUMANOID_SHIN_AXIS, HUMANOID_V0_BONES,
	HUMANOID_V0_COUNT,
};
pub use quadruped::{
	apply_quadruped_glb_rest, bone_axis as quadruped_bone_axis,
	identity_binding as quadruped_identity_binding, quadruped_v0_definition, resolve_quadruped,
	QuadrupedLimbPose, QuadrupedPose, QUADRUPED_GLB_ANTERIOR_MID_BACK, QUADRUPED_GLB_SHOULDER_L,
	QUADRUPED_GLB_THIGH, QUADRUPED_RIGHT_SHIN_AXIS, QUADRUPED_RIGHT_THIGH_AXIS,
	QUADRUPED_SHIN_AXIS, QUADRUPED_THIGH_AXIS, QUADRUPED_V0_BONES, QUADRUPED_V0_COUNT,
};
pub use space::{
	character_direction_from_world, CharacterPoint, WorldPoint, CHARACTER_FORWARD, CHARACTER_RIGHT,
	CHARACTER_UP,
};
