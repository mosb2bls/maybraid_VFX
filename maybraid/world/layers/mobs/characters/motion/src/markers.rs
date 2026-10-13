//! LOD band markers on the **host**. Sync inserts/removes them from the shown level.
//!
//! Expensive systems filter with `With<AnimateBones>` / `With<AnimateEffects>` /
//! `With<ApplyTerrainPitch>`. Do not stamp these on level-content children.

use bevy::prelude::*;

/// Write bone pose from the mailbox (`Animation::apply_for`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimateBones;

/// Write armature root-motion from the mailbox (`Animation::effects_for`).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimateEffects;

/// Stop clip output while another pose driver owns this rig.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuspendAnimation;

/// Run visual terrain pitch on this character root.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApplyTerrainPitch;

/// When present on the physics body (parent of the visual), pitch blends to 0.
///
/// Playgrounds copy jump-in-flight onto this marker.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SuspendTerrainPitch;
