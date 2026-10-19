//! Generic attack timing, hazards, and telegraphs.
//!
//! Encounter scripts and summoned owners share the same [`AttackDefinition`]
//! values. This crate does not know about Bikanwarda, Self, or any other
//! content cast. Gameplay writes [`damage::Hit`]. Presentation subscribes to
//! [`AttackPresentationEvent`] and does not own timing, collision, or Hits.
//!
//! # Stable attack / event contract
//!
//! Downstream adapters ([#1230](https://github.com/ramate-io/maybraid/pull/1230))
//! and presentation lanes ([#1150](https://github.com/ramate-io/maybraid/pull/1150),
//! [#1146](https://github.com/ramate-io/maybraid/pull/1146),
//! [#1141](https://github.com/ramate-io/maybraid/pull/1141)) should code against
//! this section. Public types, message kinds, and admission rules below are
//! the contract. The crate README repeats this section for PR readers.
//!
//! ## Entry: [`BeginAttack`]
//!
//! Write one message per requested instance. The owner must already have
//! [`AttackUser`]. Fields:
//!
//! | Field | Role |
//! |---|---|
//! | `owner` | Caster. Must exist and carry [`AttackUser`]. |
//! | `definition` | Catalog key. Shared by encounter and summoned owners. |
//! | `target` | Optional explicit victim. Aim uses combat-targeting contact memory for that subject after `CombatTargetingSystems::Rank`. Membership alone is not aimable. |
//! | `allies` | Extra friendly-fire exclusions, merged with `TargetSource::ALLY` members. |
//! | `emission` | World-space mouth / socket. Missing uses `emission_socket`, then owner origin. |
//! | `emission_socket` | Live socket sampled while the instance still tracks aim. |
//!
//! Missing contact memory falls back to owner facing. The runtime never reads
//! a live target transform and never fabricates contacts.
//!
//! ## Instance: [`AttackInstance`]
//!
//! One in-flight attack is its own entity (`attack-{id}`). The component
//! carries `id` ([`AttackInstanceId`]), `definition`, `owner`, `stage`,
//! `age` / `stage_age` / `execute_age`, [`AimSnapshot`], `live_target`,
//! `allies`, and hit clocks. Presenters key effects by `id`, not by owner.
//!
//! [`AimSnapshot`] stores three distinct world points: `origin` (caster),
//! `aim_point` (observed contact), `emission` (mouth / socket).
//! [`HazardOrigin::Owner`] and [`HazardOrigin::AimPoint`] are not the same.
//!
//! ## Stages
//!
//! [`AttackStage`] is `Windup` → `Execute` → `Recover`, then the instance
//! despawns and [`AttackUser`] starts the per-[`AttackId`] cooldown **only
//! on [`AttackEndReason::Completed`]**. Cancel / death / sleep / retirement
//! do not start cooldown.
//!
//! | Stage | Gameplay | Presentation |
//! |---|---|---|
//! | Windup | No Hits. Telegraph pose only. | `Stage { from: Windup, to: Windup }` on admit, then `Telegraph { Spawned, Updated }`. |
//! | Execute | Pulses write [`damage::Hit`]. | `Stage { Windup, Execute }`, `Hazard { Spawned, Updated }`, optional persisted telegraph. |
//! | Recover | No Hits. Volumes already cleared. | `Stage { Execute, Recover }`, `Hazard` / `Telegraph` `Despawned`. |
//!
//! A hitch that spans several stages still visits Execute so a pulse cannot
//! be skipped.
//!
//! ## Presentation: [`AttackPresentationEvent`]
//!
//! Every event carries `instance`, `owner`, `definition`, and one
//! [`AttackPresentationKind`]:
//!
//! - `Stage { from, to, snapshot }` — lifecycle, including the admit echo
//!   `Windup → Windup`.
//! - `Telegraph { phase, pose, shape }` — warning volume. Same shapes as
//!   hazards; no damage.
//! - `Hazard { index, phase, pose, shape, wave }` — gameplay volume.
//!   `wave` is `Some` only for [`HazardShape::Wavefront`].
//! - `Ended { reason }` — always paired with gameplay [`AttackEnded`].
//!
//! [`PresentationPhase`] is `Spawned` / `Updated` / `Despawned`. Spawn a
//! presenter on `Spawned`, follow `Updated`, despawn on `Despawned` or
//! `Ended`. Tag presenter entities with [`AttackPresentationBinding`] so
//! gameplay cleanup can despawn them without knowing the VFX type.
//!
//! ### Wave / hazard presentation
//!
//! [`WavefrontPresentation`] is authoritative. VFX follows it and does not
//! simulate motion: `origin`, `direction`, `position`, `speed`, `range`,
//! `radius`, `thickness` (`2 * half_depth`), `wave_index`.
//!
//! [`HazardShape::Wavefront`] is a discrete ring / short disc, not a
//! radially expanding ground ring. Rings spawn at emission, travel a
//! flattened heading (optional `pitch`), keep constant size, emit `waves`
//! at `interval`, and hit each target at most once. Collision is swept
//! Avian `shape_hits_callback` so a hitch still hits.
//!
//! Stationary hazards (`Sphere`, `Cylinder`, `Box`, `Line`) pulse on
//! Execute with per-target [`DamageSpec::cadence`] (`<= 0` means once).
//! Child colliders remount onto parent [`damage::Health`].
//!
//! ## Interruption
//!
//! [`CancelAttacks`] tears down one instance or every live instance on an
//! owner. Owner [`damage::Downed`], [`AttackSuspended`], [`AttackRetired`],
//! and owner despawn also tear down when [`InterruptionPolicy`] allows.
//! Owner despawn and completed always apply. Each teardown emits volume
//! `Despawned` events, `AttackPresentationKind::Ended`, and [`AttackEnded`],
//! then despawns the instance entity and its bindings.
//!
//! ## Concurrency
//!
//! [`AttackDefinition::concurrent`] defaults to `false`. Exclusive recipes
//! admit at most one live instance per owner. Admission is authoritative
//! for the current `begin_attacks` batch: deferred `commands.spawn` values
//! count as in-flight, so two same-owner exclusive [`BeginAttack`]s in one
//! system invocation spawn only the first that passes catalog / user /
//! cooldown gates.
//!
//! Concurrent recipes always admit (Parade child explosions
//! `bikanwarda.parade_blast`). They may overlap a live exclusive parent
//! and may stack with each other in the same batch. The exclusive lock is
//! per owner, not global.
//!
//! Cooldown is per [`AttackId`] on [`AttackUser`], checked at admit.
//!
//! ## Schedule
//!
//! `CombatTargetingSystems::Rank` → [`AttackSystems::Command`] (`BeginAttack`)
//! → `ApplyDeferred` → [`AttackSystems::Interrupt`] → `ApplyDeferred` →
//! [`AttackSystems::Advance`] (stage ticks, Hits, presentation updates).

mod aim;
mod catalog;
mod cleanup;
mod command;
mod defaults;
mod definition;
mod hazard;
mod instance;
mod overlap;
mod placeholder;
mod plugin;
mod presentation;
mod preview;
mod tick;

pub use aim::{resolve_aim_contact, AimPolicy, AimSnapshot};
pub use catalog::AttackCatalog;
pub use cleanup::{AttackRetired, AttackSuspended};
pub use command::{AttackEndReason, AttackEnded, BeginAttack, CancelAttacks};
pub use defaults::AttackDefaults;
pub use definition::{
	AttackDefinition, AttackId, AttackTiming, FriendlyFire, InterruptionPolicy, TelegraphSpec,
};
pub use hazard::{
	DamageFalloff, DamageSpec, HazardHitPass, HazardOrigin, HazardPose, HazardShape, HazardSpec,
	HitCandidate, WaveHitClock, WavefrontSpec, WavefrontSweep,
};
pub use instance::{AttackInstance, AttackInstanceId, AttackStage, AttackUser};
pub use placeholder::AttackPlaceholderPresentationPlugin;
pub use plugin::{AttackSystems, AttacksPlugin};
pub use presentation::{
	AttackPresentationBinding, AttackPresentationBindingKind, AttackPresentationEvent,
	AttackPresentationKind, PresentationPhase, WavefrontPresentation,
};
pub use preview::placeholder_svg;

#[cfg(test)]
mod runtime_tests;
