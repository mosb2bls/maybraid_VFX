//! Bridge from [`lubikans`] bindings onto generic [`damage`] credit.
//!
//! `damage` stays free of Lubikan types. This crate owns the mapping so
//! charge-credit attribution can name a [`LubikanAccountId`] after despawn
//! without a private charge model.

use bevy::prelude::*;
use damage::{maintain_damage_history, DamageActorId, DamageCredit, DamagePlugin, DamageSystems};
use lubikans::LubikanAccountBinding;

/// Copy [`LubikanAccountBinding`] onto [`DamageCredit`] before history ingest.
pub struct LubikanDamagePlugin;

impl Plugin for LubikanDamagePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<DamagePlugin>() {
			app.add_plugins(DamagePlugin);
		}
		app.add_systems(
			PostUpdate,
			attach_lubikan_damage_credits
				.in_set(DamageSystems::History)
				.before(maintain_damage_history),
		);
	}
}

/// Map a bind-time Lubikan snapshot onto the generic damage credit token.
pub fn credit_from_binding(binding: LubikanAccountBinding) -> DamageCredit {
	DamageCredit::new(
		binding.account.to_bits(),
		binding.definition.to_bits(),
		DamageActorId::from_entity(binding.owner),
	)
}

/// Recover the account packed into [`DamageCredit::account`].
pub fn account_from_credit(credit: DamageCredit) -> lubikans::LubikanAccountId {
	lubikans::LubikanAccountId::from_bits(credit.account)
}

/// Recover the definition packed into [`DamageCredit::kind`].
pub fn definition_from_credit(credit: DamageCredit) -> lubikans::LubikanDefinitionId {
	lubikans::LubikanDefinitionId::from_raw(credit.kind as u32)
}

fn attach_lubikan_damage_credits(world: &mut World) {
	let mut updates = Vec::new();
	{
		let mut bindings = world
			.query_filtered::<(Entity, &LubikanAccountBinding), Changed<LubikanAccountBinding>>();
		for (entity, binding) in bindings.iter(world) {
			updates.push((entity, credit_from_binding(*binding)));
		}
	}
	for (entity, credit) in updates {
		world.entity_mut(entity).insert(credit);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};
	use damage::{DamageActorId, DamageHistory, Health, Hit, ResponsibleOwner};
	use lubikans::{
		LubikanAccountBinding, LubikanAccountId, LubikanDefinitionId, LubikanLedger,
		LubikanSelection,
	};

	fn bridge_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, LubikanDamagePlugin));
		app
	}

	fn history(app: &App) -> Result<&DamageHistory> {
		app.world()
			.get_resource::<DamageHistory>()
			.ok_or_else(|| anyhow!("DamageHistory should be installed"))
	}

	#[test]
	fn binding_becomes_generic_credit_on_the_same_frame() -> Result<()> {
		let mut app = bridge_app();
		let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
		let attacker = app.world_mut().spawn_empty().id();
		let binding = LubikanAccountBinding::self_on(attacker);
		app.world_mut().entity_mut(attacker).insert(binding);
		app.world_mut().write_message(Hit {
			target: victim,
			source: Some(attacker),
			amount: 8.0,
			point: Vec3::ZERO,
		});
		app.update();
		let record = history(&app)?
			.for_recipient(DamageActorId::from_entity(victim))
			.next()
			.copied()
			.ok_or_else(|| anyhow!("hit should be recorded"))?;
		let credit = record.credit.ok_or_else(|| anyhow!("bridge should stamp DamageCredit"))?;
		if account_from_credit(credit) != LubikanAccountId::SELF {
			return Err(anyhow!("credit should name Self"));
		}
		if definition_from_credit(credit) != LubikanDefinitionId::SELF {
			return Err(anyhow!("kind should be Self"));
		}
		if credit.owner != DamageActorId::from_entity(attacker) {
			return Err(anyhow!("owner generation should be the attacker body"));
		}
		Ok(())
	}

	#[test]
	fn owner_binding_credits_a_summon_hit() -> Result<()> {
		let mut app = bridge_app();
		let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
		let owner = app.world_mut().spawn_empty().id();
		app.world_mut().entity_mut(owner).insert(LubikanAccountBinding::self_on(owner));
		let summon = app.world_mut().spawn(ResponsibleOwner::new(owner)).id();
		app.world_mut().write_message(Hit {
			target: victim,
			source: Some(summon),
			amount: 12.0,
			point: Vec3::ZERO,
		});
		app.update();
		let record = history(&app)?
			.for_recipient(DamageActorId::from_entity(victim))
			.next()
			.copied()
			.ok_or_else(|| anyhow!("summon hit should be recorded"))?;
		if record.source != Some(DamageActorId::from_entity(summon)) {
			return Err(anyhow!("direct attacker should be the summon"));
		}
		if record.owner != Some(DamageActorId::from_entity(owner)) {
			return Err(anyhow!("responsible owner should be the summoner"));
		}
		let credit = record.credit.ok_or_else(|| anyhow!("owner binding should credit the hit"))?;
		if account_from_credit(credit) != LubikanAccountId::SELF {
			return Err(anyhow!("charge credit should name the owner's Self account"));
		}
		Ok(())
	}

	#[test]
	fn summon_stamped_binding_survives_owner_despawn() -> Result<()> {
		let mut app = bridge_app();
		let owner = app.world_mut().spawn_empty().id();
		let binding = LubikanAccountBinding::self_account(LubikanAccountId::SELF, owner);
		let summon = app.world_mut().spawn((ResponsibleOwner::new(owner), binding)).id();
		let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
		app.world_mut().entity_mut(owner).despawn();
		app.world_mut().write_message(Hit {
			target: victim,
			source: Some(summon),
			amount: 6.0,
			point: Vec3::ZERO,
		});
		app.update();
		let record = history(&app)?
			.for_recipient(DamageActorId::from_entity(victim))
			.next()
			.copied()
			.ok_or_else(|| anyhow!("summon hit should record after owner despawn"))?;
		let credit = record.credit.ok_or_else(|| anyhow!("summon binding should remain"))?;
		if account_from_credit(credit) != LubikanAccountId::SELF {
			return Err(anyhow!("#1107 account should survive owner despawn"));
		}
		if credit.owner != DamageActorId::from_entity(owner) {
			return Err(anyhow!("generation should keep the retired owner body"));
		}
		Ok(())
	}

	#[test]
	fn selection_maps_through_ledger_binding() -> Result<()> {
		let owner = Entity::from_bits(9);
		let ledger = LubikanLedger::default();
		let from_ledger =
			credit_from_binding(LubikanAccountBinding::from_selection(ledger.selection(), owner));
		let from_resolved = credit_from_binding(LubikanAccountBinding::from_selection(
			LubikanSelection::default(),
			owner,
		));
		assert_eq!(from_ledger, from_resolved);
		assert_eq!(account_from_credit(from_ledger), LubikanAccountId::SELF);
		assert_eq!(definition_from_credit(from_ledger), LubikanDefinitionId::SELF);
		assert_eq!(from_ledger.owner, DamageActorId::from_entity(owner));
		Ok(())
	}

	#[test]
	fn owner_generation_does_not_follow_a_replaced_body() -> Result<()> {
		let first = Entity::from_bits(50);
		let replacement = Entity::from_bits(51);
		let credit = credit_from_binding(LubikanAccountBinding::self_on(first));
		assert_eq!(credit.owner, DamageActorId::from_entity(first));
		assert_ne!(credit.owner, DamageActorId::from_entity(replacement));
		assert_eq!(credit.owner.to_bits(), first.to_bits());
		Ok(())
	}
}
