//! Generic attack timing, hazards, and telegraphs.
//!
//! Producers write [`damage::Hit`]. Presentation subscribes to
//! [`AttackPresentationEvent`] and does not own gameplay.

mod aim;
mod catalog;
mod cleanup;
mod command;
mod defaults;
mod definition;
mod hazard;
mod instance;
mod placeholder;
mod plugin;
mod presentation;
mod preview;
mod tick;

pub use aim::{resolve_target, AimPolicy, AimSnapshot};
pub use catalog::AttackCatalog;
pub use cleanup::{AttackRetired, AttackSuspended};
pub use command::{AttackEndReason, AttackEnded, BeginAttack, CancelAttacks};
pub use defaults::AttackDefaults;
pub use definition::{
	AttackDefinition, AttackId, AttackTiming, FriendlyFire, InterruptionPolicy, TelegraphSpec,
};
pub use hazard::{
	DamageFalloff, DamageSpec, HazardHitPass, HazardOrigin, HazardPose, HazardShape, HazardSpec,
	HitCandidate,
};
pub use instance::{AttackInstance, AttackInstanceId, AttackStage, AttackUser};
pub use placeholder::AttackPlaceholderPresentationPlugin;
pub use plugin::{AttackSystems, AttacksPlugin};
pub use presentation::{
	AttackPresentationBinding, AttackPresentationBindingKind, AttackPresentationEvent,
	AttackPresentationKind, PresentationPhase,
};
pub use preview::placeholder_svg;

#[cfg(test)]
mod runtime_tests;
