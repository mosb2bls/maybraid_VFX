//! Reusable character definitions.
//!
//! [`CharacterComponents`] is the character recipe: species configs produce nested
//! [`lod::LodScene`] hosts ([`ComponentsOnly`], [`RigNode`], [`PartNode`]), with
//! sockets and skinning as deferred refs parallel to [`scene_ref::SceneRef`].
//! Live pose, animation, and paint are ECS mutation (`MemberOf` + `*Ref` +
//! `Changed`), not LOD refresh. Runtime world/mob visuals spawn a **fixed
//! assembly** ([`fixed`]) — ordinary [`CharacterRoot`] / rig / part entities
//! without query-only [`lod::LodSceneHost`]s. Playground previews still use
//! [`ComponentsOnly`] [`lod::LodScene::host`]. Recipes also expose a rest-pose
//! [`LocomotionCapsule`](crate::LocomotionCapsule) for locomotion colliders;
//! physics crates stamp Avian from that hull. Pronograde recipes also expose a
//! query-only [`HitCapsule`](crate::HitCapsule) (horizontal, along mesh `+Z`);
//! hit girth is rest-pose shoulder / hip / torso bone scale. Biped recipes keep
//! one vertical motor capsule sized from rest-pose legs / spine / neck / shoulders.
//! Oversized heads add a query-only [`HeadCapsule`](crate::HeadCapsule) that
//! stretches on Y above that hull. The motor stays on the vertical capsule. Per-frame clips and terrain pitch live in
//! [`character_motion`]; this crate stamps **initial** host markers from
//! [`lod::LodScene::host`] / spawn. Motion sync keeps those markers aligned with
//! the shown LOD band.

pub mod anim;
pub mod appearance;
pub mod assembly;
pub mod assets;
pub mod components;
pub mod concepts;
pub mod fixed;
pub mod hosts;
pub mod layer;
pub mod material_lib;
pub mod member;
pub mod menu_traits;
pub mod nodes;
pub mod plugin;
pub mod pose;
pub mod presets;
pub mod rig;
pub mod scene_children;
pub mod skin;
pub mod socket;
pub mod species;
pub mod terrain_pitch;

pub use anim::{
	apply_anim_mailbox, prepare_anim_mailbox, tick_anim_mailbox, AnimBone, AnimClip, AnimId,
	AnimMailbox, AnimProgress, AnimRef, AnimRefRoot, JabParams, JumpParams, TuckParams,
	TuckedFlipParams, TwoFootedTuckedFlipParams,
};
pub use appearance::CharacterAppearance;
pub use assembly::CharacterPartSlot;
pub use assets::{AssetFacing, AssetNormalization, AssetPath, AuthoredAnchor};
pub use character_motion::{
	apply_terrain_pitch, clamp_intelligence, draw_terrain_pitch_probes, motion_policy,
	sync_motion_markers, AnimateBones, AnimateEffects, ApplyTerrainPitch, CharacterHeading,
	CharacterMotionPlugin, CharacterMotionSystems, DrawTerrainPitchProbes, MotionPolicy,
	SuspendAnimation, SuspendTerrainPitch,
};
pub use character_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};
pub use character_shaders::CharacterShadersPlugin;
pub use components::{
	character_bounds, clothing_layers, CharacterComponents, CharacterRecipe, Clothed,
	ClothingLayer, ComponentsOnly, HeadCapsule, HitCapsule, LocomotionCapsule,
};
pub use concepts::ConceptAnimation;
pub use fixed::{
	drain_character_assembly, fixed_character_assembly_chunks, spawn_fixed_character_assembly,
	spawn_fixed_character_visual, CharacterAssemblyBudget, CharacterAssemblyDiagnostics,
	PendingCharacterAssembly, FIXED_ASSEMBLY_CHUNK_WEIGHT,
};
pub use hosts::CharacterHostsPlugin;
pub use layer::{Layer, Layers};
pub use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
pub use member::{
	attach_part_node, find_member_rig, find_part_member, hide_socketed_parts,
	stamp_character_members, CharacterMembers, CharacterRoot, MemberOf,
};
pub use nodes::{PartNode, RigNode};
pub use plugin::{add_character_components_host, CharacterComponentsPlugin, CharacterHostSystems};
pub use pose::maintain_resolved_pose;
pub use presets::{BuildPreset, GenderPreset};
pub use rig::{
	bind_scales_ready, bone_map_ready, build_rig_bone_map, missing_landmark_bones, ActiveRigPose,
	BoneMap, CharacterPart, CharacterRig, CharacterRigRole, LodCharacterRig,
	NeedsDuplicateScenePrune, NeedsSkinRemap, NoMatchingArmature, PartRigRef, ResolvedPoseApplied,
	RigBindScales, RigSkeletonKind,
};
pub use rigs::{AssemblyHost, AssemblyRoot, RigKey, RigPlugin, RigRoot, RigSystems};
pub use skin::{
	fulfill_skin_ref_roots, invalidate_changed_skin_ref_roots, prune_duplicate_part_scenes,
	remap_part_skin_to_rig,
};
pub use socket::{
	fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots, RigId, SkinRef, SkinRefApplied,
	SkinRefRoot, SocketRef, SocketRefApplied, SocketRefRoot,
};
pub use terrain_pitch::prepare_character_terrain_pitch;
pub use terrain_pitch::TerrainPitch;
