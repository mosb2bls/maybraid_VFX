//! Bikanwarda boss character assembly ([#1134](https://github.com/ramate-io/maybraid/issues/1134)).
//!
//! Wraps the Epiphant [`CharacterRecipe`](characters::CharacterRecipe) with encounter
//! scale, form presentation (Encounter / Ward / Enraged), chain anchors, and collider
//! fit derived from the locomotion hull. Bray and sleep/wake reuse the shared quadruped
//! clips ([#1130](https://github.com/ramate-io/maybraid/issues/1130)); this crate does
//! not duplicate assembly or mailbox systems.

pub mod anchors;
pub mod animation;
pub mod bounds;
pub mod colliders;
pub mod measure;
pub mod presentation;
pub mod recipe;
pub mod spawn;
pub mod spec;

pub use anchors::{
	BikanwardaAnchors, BikanwardaChainAnchor, CHAIN_ANCHOR_NAMES, CHAIN_SOCKET_NAMES,
};
pub use animation::{animated_bounds_envelope, AnimationBoundsSample, BIKANWARDA_ANIMATION_SAMPLES};
pub use bounds::visual_bounds_aabb;
pub use colliders::{assembly_collider_shapes, motor_collider};
pub use measure::{
	encounter_uniform_scale, ordinary_standing_height_m, standing_height_m,
	ENCOUNTER_STANDING_HEIGHT_M, STANDING_HEIGHT_TOLERANCE_M,
};
pub use presentation::BikanwardaPresentation;
pub use recipe::{BikanwardaActor, BikanwardaActorRecipe};
pub use spawn::{
	spawn_bikanwarda_encounter, spawn_bikanwarda_enraged, spawn_bikanwarda_ward,
	BikanwardaEncounter, BikanwardaEnraged, BikanwardaRoot, BikanwardaWard,
};
pub use spec::{BikanwardaForm, BikanwardaSpec};
