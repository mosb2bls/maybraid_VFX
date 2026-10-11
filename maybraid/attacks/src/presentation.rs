use bevy::prelude::*;

use crate::{
	AimSnapshot, AttackEndReason, AttackId, AttackInstanceId, AttackStage, HazardPose, HazardShape,
};

/// Gameplay announcement. VFX crates subscribe; they do not own timing or Hits.
#[derive(Message, Clone, Debug)]
pub struct AttackPresentationEvent {
	pub instance: AttackInstanceId,
	pub owner: Entity,
	pub definition: AttackId,
	pub kind: AttackPresentationKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttackPresentationKind {
	Stage { from: AttackStage, to: AttackStage, snapshot: AimSnapshot },
	Telegraph { phase: PresentationPhase, pose: HazardPose, shape: HazardShape },
	Hazard { index: usize, phase: PresentationPhase, pose: HazardPose, shape: HazardShape },
	Ended { reason: AttackEndReason },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPhase {
	Spawned,
	Updated,
	Despawned,
}

/// Tag a presenter-owned entity so cancellation, death, sleep, and retirement
/// can despawn it without knowing the VFX type.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackPresentationBinding {
	pub instance: AttackInstanceId,
	pub kind: AttackPresentationBindingKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackPresentationBindingKind {
	Telegraph,
	Hazard { index: usize },
}

impl AttackPresentationEvent {
	pub fn ended(
		instance: AttackInstanceId,
		owner: Entity,
		definition: AttackId,
		reason: AttackEndReason,
	) -> Self {
		Self { instance, owner, definition, kind: AttackPresentationKind::Ended { reason } }
	}

	pub fn with_kind(
		instance: AttackInstanceId,
		owner: Entity,
		definition: AttackId,
		kind: AttackPresentationKind,
	) -> Self {
		Self { instance, owner, definition, kind }
	}
}
