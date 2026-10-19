use bevy::prelude::*;

use crate::{
	AimSnapshot, AttackEndReason, AttackId, AttackInstanceId, AttackStage, HazardPose, HazardShape,
};
use crate::hazard::WavefrontSpec;

/// Gameplay announcement. VFX crates subscribe; they do not own timing or Hits.
///
/// Kinds: `Stage`, `Telegraph`, `Hazard` (optional [`WavefrontPresentation`]),
/// and `Ended`. Bind presenter entities with [`AttackPresentationBinding`] so
/// interruption can despawn them.
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
	Hazard {
		index: usize,
		phase: PresentationPhase,
		pose: HazardPose,
		shape: HazardShape,
		wave: Option<WavefrontPresentation>,
	},
	Ended { reason: AttackEndReason },
}

/// Authoritative per-wave state. Presenters follow this; they do not simulate motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WavefrontPresentation {
	/// Mouth / socket at emit time.
	pub origin: Vec3,
	/// World-space travel heading.
	pub direction: Vec3,
	/// Current ring center.
	pub position: Vec3,
	pub speed: f32,
	pub range: f32,
	pub radius: f32,
	/// Full thickness along travel (`2 * half_depth`).
	pub thickness: f32,
	pub wave_index: u8,
}

impl WavefrontPresentation {
	pub fn from_wave(
		wave: WavefrontSpec,
		emission: Vec3,
		heading: Vec3,
		travel: f32,
		wave_index: u8,
	) -> Self {
		let pose = wave.pose(emission, heading, travel);
		Self {
			origin: emission,
			direction: heading,
			position: pose.origin,
			speed: wave.speed,
			range: wave.range,
			radius: wave.radius,
			thickness: wave.half_depth.max(0.0) * 2.0,
			wave_index,
		}
	}
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
	Hazard { index: usize, wave: u8 },
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
