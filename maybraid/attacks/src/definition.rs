use bevy::prelude::Entity;

use crate::{AimPolicy, AttackDefaults, DamageSpec, HazardSpec};

/// Stable catalog key. Encounter and summoned owners share the same ids.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttackId(pub &'static str);

impl AttackId {
	pub const fn new(id: &'static str) -> Self {
		Self(id)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}
}

/// Stage durations in seconds. Cooldown is stored on [`crate::AttackUser`]
/// after the instance completes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackTiming {
	pub windup: f32,
	pub execution: f32,
	pub recovery: f32,
	pub cooldown: f32,
}

impl AttackTiming {
	pub fn new(windup: f32, execution: f32, recovery: f32, cooldown: f32) -> Self {
		Self {
			windup: windup.max(0.0),
			execution: execution.max(0.0),
			recovery: recovery.max(0.0),
			cooldown: cooldown.max(0.0),
		}
	}

	/// Windup, execution, and recovery from the caller; cooldown from defaults.
	pub fn with_authored_cooldown(windup: f32, execution: f32, recovery: f32) -> Self {
		Self::new(windup, execution, recovery, AttackDefaults::AUTHORED.cooldown)
	}

	pub fn duration(self, stage: crate::AttackStage) -> f32 {
		match stage {
			crate::AttackStage::Windup => self.windup,
			crate::AttackStage::Execute => self.execution,
			crate::AttackStage::Recover => self.recovery,
		}
	}
}

/// Which owner-lifecycle events cancel a live instance.
///
/// Owner despawn always cleans up, and is not configurable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterruptionPolicy {
	pub cancel: bool,
	pub downed: bool,
	pub suspended: bool,
	pub retired: bool,
}

impl InterruptionPolicy {
	/// Authored default: cancel, death, sleep, and retirement all tear down.
	pub const AUTHORED: Self = Self { cancel: true, downed: true, suspended: true, retired: true };

	pub const fn allows(self, reason: crate::AttackEndReason) -> bool {
		match reason {
			crate::AttackEndReason::Cancelled => self.cancel,
			crate::AttackEndReason::OwnerDowned => self.downed,
			crate::AttackEndReason::OwnerSuspended => self.suspended,
			crate::AttackEndReason::OwnerRetired => self.retired,
			crate::AttackEndReason::OwnerDespawned | crate::AttackEndReason::Completed => true,
		}
	}
}

/// Who may receive hazard [`damage::Hit`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FriendlyFire {
	/// Hit everyone, including the owner.
	All,
	/// Never hit the owner.
	ExcludeOwner,
	/// Never hit the owner or allies listed on the instance / targeting.
	ExcludeOwnerAndAllies,
}

impl FriendlyFire {
	pub fn admits(self, owner: Entity, target: Entity, allies: &[Entity]) -> bool {
		match self {
			Self::All => true,
			Self::ExcludeOwner => target != owner,
			Self::ExcludeOwnerAndAllies => target != owner && !allies.contains(&target),
		}
	}
}

/// Warning geometry shown during windup. Same shapes as hazards; no damage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TelegraphSpec {
	pub shape: crate::HazardShape,
	pub origin: crate::HazardOrigin,
	/// Keep the warning visible while the hazard is live.
	pub persist_into_execute: bool,
}

/// Immutable attack recipe. Shared by encounter and summoned owners.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackDefinition {
	pub id: AttackId,
	pub timing: AttackTiming,
	pub aim: AimPolicy,
	pub interruption: InterruptionPolicy,
	pub friendly_fire: FriendlyFire,
	pub telegraph: Option<TelegraphSpec>,
	pub hazards: Vec<HazardSpec>,
	/// When false, a second begin is ignored until the live instance ends.
	pub concurrent: bool,
}

impl AttackDefinition {
	/// Every policy field is required. Use [`Self::with_authored_defaults`] to
	/// copy [`AttackDefaults::AUTHORED`].
	pub fn new(
		id: AttackId,
		timing: AttackTiming,
		aim: AimPolicy,
		interruption: InterruptionPolicy,
		friendly_fire: FriendlyFire,
	) -> Self {
		Self {
			id,
			timing,
			aim,
			interruption,
			friendly_fire,
			telegraph: None,
			hazards: Vec::new(),
			concurrent: false,
		}
	}

	/// Shared recipe skeleton for encounter and summoned owners.
	pub fn with_authored_defaults(
		id: AttackId,
		windup: f32,
		execution: f32,
		recovery: f32,
		aim: AimPolicy,
	) -> Self {
		let defaults = AttackDefaults::AUTHORED;
		Self::new(
			id,
			AttackTiming::with_authored_cooldown(windup, execution, recovery),
			aim,
			defaults.interruption,
			defaults.friendly_fire,
		)
	}

	pub fn with_telegraph(mut self, telegraph: TelegraphSpec) -> Self {
		self.telegraph = Some(telegraph);
		self
	}

	pub fn with_hazard(mut self, hazard: HazardSpec) -> Self {
		self.hazards.push(hazard);
		self
	}

	pub fn with_concurrent(mut self, concurrent: bool) -> Self {
		self.concurrent = concurrent;
		self
	}

	pub fn authored_hazard(self, shape: crate::HazardShape, origin: crate::HazardOrigin) -> Self {
		self.with_hazard(HazardSpec { shape, origin, damage: DamageSpec::authored() })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{HazardOrigin, HazardShape};
	use bevy::prelude::Entity;

	#[test]
	fn timing_clamps_negative_durations() {
		let timing = AttackTiming::new(-1.0, -2.0, -3.0, -4.0);
		assert_eq!(timing.windup, 0.0);
		assert_eq!(timing.execution, 0.0);
		assert_eq!(timing.recovery, 0.0);
		assert_eq!(timing.cooldown, 0.0);
	}

	#[test]
	fn authored_definition_copies_published_defaults() {
		let def = AttackDefinition::with_authored_defaults(
			AttackId::new("column"),
			1.5,
			3.0,
			1.0,
			AimPolicy::TargetAtWindup,
		)
		.authored_hazard(
			HazardShape::Cylinder { radius: 1.5, height: 7.0 },
			HazardOrigin::AimPoint,
		);
		assert_eq!(def.timing.cooldown, AttackDefaults::AUTHORED.cooldown);
		assert_eq!(def.interruption, AttackDefaults::AUTHORED.interruption);
		assert_eq!(def.friendly_fire, AttackDefaults::AUTHORED.friendly_fire);
		assert_eq!(def.hazards[0].damage.amount, AttackDefaults::AUTHORED.damage);
		assert_eq!(def.hazards[0].damage.cadence, AttackDefaults::AUTHORED.cadence);
	}

	#[test]
	fn friendly_fire_excludes_owner_and_allies() {
		let owner = Entity::from_bits(1);
		let ally = Entity::from_bits(2);
		let foe = Entity::from_bits(3);
		let policy = FriendlyFire::ExcludeOwnerAndAllies;
		assert!(!policy.admits(owner, owner, &[ally]));
		assert!(!policy.admits(owner, ally, &[ally]));
		assert!(policy.admits(owner, foe, &[ally]));
	}
}
