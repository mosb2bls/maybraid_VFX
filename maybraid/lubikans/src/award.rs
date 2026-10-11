//! Award path: one request applies once and emits one [`LubikanChargeChanged`].

use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems, Health};
use player::Player;

use crate::change::{LubikanChargeChanged, LubikanChargeReason, LubikanChargeRequest};
use crate::ledger::{LubikanLedger, LubikanOwner};
use crate::LubikanSystems;

type LiveLubikanOwner = Or<(With<Player>, With<LubikanOwner>)>;

/// Combat → charge. Hits credit applied HP; kills credit opponent max health.
///
/// Default multipliers are 1. A lethal hit still writes the hit credit when
/// [`Self::stack_kill`] is set, then a separate kill credit.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanCombatAward {
	/// Multiplier on applied HP for a hit credit.
	pub hit: f32,
	/// Multiplier on opponent max health for a kill credit.
	pub kill: f32,
	/// When true, a lethal hit writes both the hit credit and the kill credit.
	pub stack_kill: bool,
}

impl Default for LubikanCombatAward {
	fn default() -> Self {
		Self { hit: 1.0, kill: 1.0, stack_kill: true }
	}
}

/// Per-apply combat credits. Zero-effect hits yield both `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LubikanCombatCredits {
	pub hit: Option<f32>,
	pub kill: Option<f32>,
}

impl LubikanCombatAward {
	/// Credits for one [`DamageApplied`].
	///
	/// `opponent_health` is the target's max HP. Zero-effect hits award
	/// nothing. Overkill uses [`DamageApplied::applied`], not requested
	/// `amount`, so a 100-damage shot into 3 HP cannot over-award the hit.
	pub fn credits_for(self, hit: DamageApplied, opponent_health: f32) -> LubikanCombatCredits {
		if hit.is_zero_effect() {
			return LubikanCombatCredits::default();
		}
		let lethal = hit.remaining <= 0.0;
		let dealt = (hit.applied * self.hit).max(0.0);
		let health = (opponent_health.max(0.0) * self.kill).max(0.0);
		let hit_credit = if !lethal || self.stack_kill { Some(dealt).filter(|value| *value > 0.0) } else { None };
		let kill_credit = if lethal { Some(health).filter(|value| *value > 0.0) } else { None };
		LubikanCombatCredits { hit: hit_credit, kill: kill_credit }
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
	healths: Query<&Health>,
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
		let opponent_health = healths
			.get(hit.target)
			.map(|health| health.max)
			.unwrap_or_else(|_| (hit.applied + hit.remaining.max(0.0)).max(0.0));
		let credits = policy.credits_for(*hit, opponent_health);
		if let Some(requested) = credits.hit {
			requests.write(LubikanChargeRequest::equipped(
				source,
				requested,
				LubikanChargeReason::COMBAT_HIT,
				Some(hit.point),
			));
		}
		if let Some(requested) = credits.kill {
			requests.write(LubikanChargeRequest::equipped(
				source,
				requested,
				LubikanChargeReason::COMBAT_KILL,
				Some(hit.point),
			));
		}
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
		assert_eq!(policy.credits_for(applied(50.0, 0.0, 10.0), 10.0), LubikanCombatCredits::default());
		assert_eq!(
			policy.credits_for(applied(50.0, 3.0, 0.0), 10.0),
			LubikanCombatCredits { hit: Some(3.0), kill: Some(10.0) }
		);
		assert_eq!(
			policy.credits_for(applied(10.0, 10.0, 4.0), 14.0),
			LubikanCombatCredits { hit: Some(10.0), kill: None }
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
		let victim = app
			.world_mut()
			.spawn((Health { current: 5.0, max: 5.0 }, GlobalTransform::default()))
			.id();
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
		assert_eq!(ledger.equipped().charge().current(), 10.0);
		Ok(())
	}
}
