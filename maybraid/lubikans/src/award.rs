//! Award path: one request applies once and emits one [`LubikanChargeChanged`].

use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems, HeadshotBand};
use player::Player;

use crate::change::{LubikanChargeChanged, LubikanChargeReason, LubikanChargeRequest};
use crate::ledger::{LubikanLedger, LubikanOwner};
use crate::LubikanSystems;

type LiveLubikanOwner = Or<(With<Player>, With<LubikanOwner>)>;

/// Configurable combat → charge weights. Defaults are +10 hit / +25 kill;
/// a lethal hit stacks both.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanCombatAward {
	pub hit: f32,
	pub headshot: f32,
	pub kill: f32,
	/// When true, a lethal hit adds the kill weight on top of the hit weight.
	pub stack_kill: bool,
}

impl Default for LubikanCombatAward {
	fn default() -> Self {
		Self { hit: 10.0, headshot: 10.0, kill: 25.0, stack_kill: true }
	}
}

impl LubikanCombatAward {
	/// Charge for one [`DamageApplied`]. Zero-effect hits award nothing.
	/// Overkill uses the applied HP only as a gate — requested `amount` is
	/// ignored so a 100-damage shot into 3 HP cannot over-award.
	pub fn charge_for(
		self,
		hit: DamageApplied,
		headshot: bool,
	) -> Option<(f32, LubikanChargeReason)> {
		if hit.is_zero_effect() {
			return None;
		}
		let lethal = hit.remaining <= 0.0;
		let hit_weight = if headshot { self.headshot } else { self.hit };
		let (requested, reason) = if lethal && self.stack_kill {
			(hit_weight + self.kill, LubikanChargeReason::COMBAT_KILL)
		} else if lethal {
			(self.kill, LubikanChargeReason::COMBAT_KILL)
		} else {
			(hit_weight, LubikanChargeReason::COMBAT_HIT)
		};
		Some((requested, reason))
	}
}

pub(crate) fn configure(app: &mut App) {
	if app.world().get_resource::<Messages<DamageApplied>>().is_none() {
		app.add_message::<DamageApplied>();
	}
	app.init_resource::<LubikanCombatAward>()
		.configure_sets(PostUpdate, LubikanSystems::Award.after(DamageSystems::Apply));
	app.add_systems(
		PostUpdate,
		(award_combat_hits, apply_charge_requests).chain().in_set(LubikanSystems::Award),
	);
}

fn award_combat_hits(
	mut hits: MessageReader<DamageApplied>,
	mut requests: MessageWriter<LubikanChargeRequest>,
	policy: Res<LubikanCombatAward>,
	owners: Query<Entity, LiveLubikanOwner>,
	targets: Query<(&GlobalTransform, Option<&HeadshotBand>)>,
) {
	for hit in hits.read() {
		let Some(source) = hit.source else {
			continue;
		};
		if owners.get(source).is_err() {
			continue;
		}
		if owners.get(hit.target).is_ok() {
			continue;
		}
		let headshot = targets.get(hit.target).ok().is_some_and(|(transform, band)| {
			band.is_some_and(|band| band.contains(transform, hit.point))
		});
		let Some((requested, reason)) = policy.charge_for(*hit, headshot) else {
			continue;
		};
		requests.write(LubikanChargeRequest::equipped(source, requested, reason, Some(hit.point)));
	}
}

fn apply_charge_requests(
	mut requests: MessageReader<LubikanChargeRequest>,
	mut ledgers: Query<&mut LubikanLedger>,
	mut changed: MessageWriter<LubikanChargeChanged>,
) {
	for request in requests.read() {
		let Ok(mut ledger) = ledgers.get_mut(request.recipient) else {
			continue;
		};
		let account = request.lubikan.unwrap_or_else(|| ledger.selection().resolved());
		let Some(mutation) = ledger.apply_to(account, request.requested) else {
			continue;
		};
		changed.write(LubikanChargeChanged::new(
			request.recipient,
			account,
			mutation.applied,
			request.reason,
			request.origin,
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::plugin::LubikansPlugin;
	use crate::{LubikanAccountId, LubikanOwner};
	use anyhow::{anyhow, Result};

	fn gameplay() -> App {
		let mut app = App::new();
		app.add_plugins(LubikansPlugin);
		app
	}

	fn applied(amount: f32, lost: f32, remaining: f32) -> DamageApplied {
		DamageApplied {
			target: Entity::from_bits(2),
			source: Some(Entity::from_bits(1)),
			amount,
			applied: lost,
			remaining,
			point: Vec3::ZERO,
		}
	}

	#[test]
	fn zero_effect_and_overkill_use_applied_hp() {
		let policy = LubikanCombatAward::default();
		assert_eq!(policy.charge_for(applied(50.0, 0.0, 10.0), false), None);
		assert_eq!(
			policy.charge_for(applied(50.0, 3.0, 0.0), false),
			Some((35.0, LubikanChargeReason::COMBAT_KILL))
		);
		assert_eq!(
			policy.charge_for(applied(10.0, 10.0, 4.0), true),
			Some((10.0, LubikanChargeReason::COMBAT_HIT))
		);
	}

	#[test]
	fn occupancy_shaped_request_applies_once() -> Result<()> {
		let mut app = gameplay();
		let owner = app.world_mut().spawn(LubikanOwner).id();
		app.update();
		app.world_mut().write_message(LubikanChargeRequest::new(
			owner,
			Some(LubikanAccountId::SELF),
			4.0,
			LubikanChargeReason::OCCUPANCY,
			Some(Vec3::X),
		));
		app.update();
		let change = app
			.world_mut()
			.resource_mut::<Messages<LubikanChargeChanged>>()
			.drain()
			.find(|change| change.reason == LubikanChargeReason::OCCUPANCY)
			.ok_or_else(|| anyhow!("occupancy change"))?;
		assert_eq!(change.recipient, owner);
		assert_eq!(change.lubikan, LubikanAccountId::SELF);
		assert_eq!(change.amount, 4.0);
		assert_eq!(change.origin, Some(Vec3::X));
		let ledger = app.world().get::<LubikanLedger>(owner).ok_or_else(|| anyhow!("ledger"))?;
		assert_eq!(ledger.equipped().charge().current(), 4.0);
		Ok(())
	}

	#[test]
	fn combat_hit_awards_equipped_and_skips_zero_effect() -> Result<()> {
		let mut app = gameplay();
		let owner = app.world_mut().spawn((Player, LubikanOwner)).id();
		let victim = app.world_mut().spawn(GlobalTransform::default()).id();
		app.update();
		app.world_mut().write_message(DamageApplied {
			target: victim,
			source: Some(owner),
			amount: 80.0,
			applied: 0.0,
			remaining: 5.0,
			point: Vec3::ZERO,
		});
		app.update();
		let empty = app
			.world()
			.get::<LubikanLedger>(owner)
			.ok_or_else(|| anyhow!("ledger"))?
			.equipped()
			.charge()
			.current();
		assert_eq!(empty, 0.0);
		app.world_mut().write_message(DamageApplied {
			target: victim,
			source: Some(owner),
			amount: 80.0,
			applied: 5.0,
			remaining: 0.0,
			point: Vec3::ZERO,
		});
		app.update();
		let ledger = app.world().get::<LubikanLedger>(owner).ok_or_else(|| anyhow!("ledger"))?;
		assert_eq!(ledger.equipped().charge().current(), 35.0);
		Ok(())
	}
}
