use anyhow::{anyhow, Result};
use bevy::prelude::*;
use projectiles::ProjectileContact;

use crate::{
	maintain_damage_history, ClearDamageHistory, DamageActorId, DamageApplied, DamageCredit,
	DamageHistory, DamageHistoryPolicy, DamagePlugin, DamageRecord, DamageSystems, Health, Hit,
	ResponsibleOwner, TimeInterval,
};

fn history_app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, DamagePlugin))
		.add_message::<ProjectileContact>();
	app
}

fn write_applied(app: &mut App, target: Entity, source: Option<Entity>, amount: f32) {
	app.world_mut().write_message(DamageApplied {
		target,
		source,
		amount,
		applied: amount,
		remaining: 100.0 - amount,
		point: Vec3::ZERO,
	});
}

fn record(at: f32, recipient: u64, requested: f32, applied: f32, remaining: f32) -> DamageRecord {
	DamageRecord {
		at,
		recipient: DamageActorId::from_bits(recipient),
		requested,
		applied,
		remaining,
		point: Vec3::ZERO,
		source: None,
		owner: None,
		credit: None,
	}
}

fn history(app: &App) -> Result<&DamageHistory> {
	app.world()
		.get_resource::<DamageHistory>()
		.ok_or_else(|| anyhow!("DamageHistory should be installed"))
}

fn set_elapsed(app: &mut App, secs: f32) {
	let mut time = app.world_mut().resource_mut::<Time<Virtual>>();
	time.advance_to(std::time::Duration::from_secs_f32(secs.max(0.0)));
}

#[test]
fn interval_boundaries_are_closed() -> Result<()> {
	let mut history = DamageHistory::default();
	history.push(record(1.0, 1, 4.0, 4.0, 96.0));
	history.push(record(2.0, 1, 5.0, 5.0, 91.0));
	history.push(record(3.0, 1, 6.0, 6.0, 85.0));
	let recipient = DamageActorId::from_bits(1);
	let in_range: Vec<f32> = history
		.for_recipient_in(recipient, TimeInterval::new(2.0, 3.0))
		.map(|record| record.at)
		.collect();
	assert_eq!(in_range, vec![2.0, 3.0]);
	assert!(TimeInterval::new(2.0, 2.0).contains(2.0));
	assert!(!TimeInterval::new(2.0, 2.0).contains(1.999));
	assert!(!TimeInterval::new(2.0, 3.0).contains(3.001));
	assert_eq!(history.for_recipient_in(recipient, TimeInterval::new(1.5, 1.9)).count(), 0);
	assert!(history.received_positive_damage_since(recipient, 3.0));
	assert!(!history.received_positive_damage_since(recipient, 3.1));
	Ok(())
}

#[test]
fn healing_lookback_uses_applied_hp_not_requested() -> Result<()> {
	let recipient = DamageActorId::from_bits(7);
	let mut history = DamageHistory::default();
	history.push(record(4.0, 7, 8.0, 0.0, 100.0));
	assert!(!history.received_positive_damage_since(recipient, 1.0));
	assert!(history.records().next().is_some_and(|record| record.is_zero_effect()));
	history.push(record(5.0, 7, 1.0, 1.0, 99.0));
	assert!(history.received_positive_damage_since(recipient, 5.0));
	assert!(!history.received_positive_damage_since(DamageActorId::from_bits(8), 1.0));
	Ok(())
}

#[test]
fn multiple_hits_in_one_frame_are_retained() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	let attacker = app.world_mut().spawn_empty().id();
	write_applied(&mut app, victim, Some(attacker), 3.0);
	write_applied(&mut app, victim, Some(attacker), 5.0);
	app.update();
	let recorded: Vec<f32> = history(&app)?
		.for_recipient(DamageActorId::from_entity(victim))
		.map(|record| record.applied)
		.collect();
	assert_eq!(recorded, vec![3.0, 5.0]);
	Ok(())
}

#[test]
fn source_despawn_keeps_generic_credit() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	let owner = Entity::from_bits(11);
	let credit = DamageCredit::from_owner_entity(3, 0, owner);
	let attacker = app.world_mut().spawn(credit).id();
	let attacker_id = DamageActorId::from_entity(attacker);
	write_applied(&mut app, victim, Some(attacker), 8.0);
	app.update();
	app.world_mut().entity_mut(attacker).despawn();
	app.update();
	let record = history(&app)?
		.for_recipient(DamageActorId::from_entity(victim))
		.next()
		.copied()
		.ok_or_else(|| anyhow!("recorded hit should survive source despawn"))?;
	if record.source != Some(attacker_id) {
		return Err(anyhow!(
			"source identity should remain {attacker_id:?}, got {:?}",
			record.source
		));
	}
	if record.credit != Some(credit) {
		return Err(anyhow!("generic credit should remain after source despawn"));
	}
	assert!(!app.world().entities().contains(attacker));
	Ok(())
}

#[test]
fn recycled_entity_does_not_match_retired_recipient() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	write_applied(&mut app, victim, None, 9.0);
	app.update();
	let retired = DamageActorId::from_entity(victim);
	app.world_mut().entity_mut(victim).despawn();
	let recycled = app.world_mut().spawn(Health::from_max(100.0)).id();
	app.update();
	let hist = history(&app)?;
	assert!(hist.received_positive_damage_since(retired, 0.0));
	assert!(!hist.received_positive_damage_since(DamageActorId::from_entity(recycled), 0.0));
	if retired == DamageActorId::from_entity(recycled) {
		return Err(anyhow!("recycled occupant must not reuse the retired actor id"));
	}
	Ok(())
}

#[test]
fn capacity_overflow_drops_oldest() -> Result<()> {
	let mut history = DamageHistory::default();
	for index in 0..4 {
		history.push(record(index as f32, 3, 1.0 + index as f32, 1.0 + index as f32, 90.0));
	}
	history.evict(10.0, DamageHistoryPolicy { retain_secs: 60.0, max_records: 2 });
	let amounts: Vec<f32> = history.records().map(|record| record.applied).collect();
	assert_eq!(amounts, vec![3.0, 4.0]);
	Ok(())
}

#[test]
fn expired_records_are_removed() -> Result<()> {
	let mut history = DamageHistory::default();
	history.push(record(1.0, 4, 2.0, 2.0, 98.0));
	history.push(record(10.0, 4, 3.0, 3.0, 95.0));
	history.evict(12.0, DamageHistoryPolicy { retain_secs: 5.0, max_records: 64 });
	let amounts: Vec<f32> = history.records().map(|record| record.applied).collect();
	assert_eq!(amounts, vec![3.0]);
	Ok(())
}

#[test]
fn session_reset_clears_history() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	write_applied(&mut app, victim, None, 4.0);
	app.update();
	if history(&app)?.is_empty() {
		return Err(anyhow!("hit should be retained before reset"));
	}
	app.world_mut().write_message(ClearDamageHistory);
	app.update();
	if !history(&app)?.is_empty() {
		return Err(anyhow!("session reset should drop retained records"));
	}
	Ok(())
}

#[test]
fn responsible_owner_is_distinct_from_direct_attacker() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	let owner = app.world_mut().spawn_empty().id();
	let credit = DamageCredit::from_owner_entity(9, 0, owner);
	app.world_mut().entity_mut(owner).insert(credit);
	let summon = app.world_mut().spawn(ResponsibleOwner::new(owner)).id();
	write_applied(&mut app, victim, Some(summon), 12.0);
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
	if record.credit != Some(credit) {
		return Err(anyhow!("credit on the owner should attach through ResponsibleOwner"));
	}
	Ok(())
}

#[test]
fn credit_on_summon_survives_owner_despawn() -> Result<()> {
	let mut app = history_app();
	let owner = app.world_mut().spawn_empty().id();
	let credit = DamageCredit::from_owner_entity(9, 0, owner);
	let summon = app.world_mut().spawn((ResponsibleOwner::new(owner), credit)).id();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	app.world_mut().entity_mut(owner).despawn();
	write_applied(&mut app, victim, Some(summon), 6.0);
	app.update();
	let record = history(&app)?
		.for_recipient(DamageActorId::from_entity(victim))
		.next()
		.copied()
		.ok_or_else(|| anyhow!("summon hit should record after owner despawn"))?;
	if record.owner != Some(DamageActorId::from_entity(owner)) {
		return Err(anyhow!("owner actor id should stay on the record"));
	}
	if record.credit != Some(credit) {
		return Err(anyhow!("summon-stamped credit should survive owner despawn"));
	}
	Ok(())
}

#[test]
fn overkill_records_applied_hp_separately_from_requested() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(10.0)).id();
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 25.0,
		point: Vec3::ZERO,
	});
	app.update();
	let record = history(&app)?
		.for_recipient(DamageActorId::from_entity(victim))
		.next()
		.copied()
		.ok_or_else(|| anyhow!("overkill hit should be recorded"))?;
	if record.requested != 25.0 {
		return Err(anyhow!("requested should stay 25, got {}", record.requested));
	}
	if record.applied != 10.0 {
		return Err(anyhow!("applied should be remaining HP 10, got {}", record.applied));
	}
	if record.remaining != 0.0 || !record.is_overkill() || !record.is_positive() {
		return Err(anyhow!("overkill should be positive applied HP, not the request"));
	}
	Ok(())
}

#[test]
fn already_dead_hits_do_not_enter_history() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(10.0)).id();
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 10.0,
		point: Vec3::ZERO,
	});
	app.update();
	if history(&app)?.len() != 1 {
		return Err(anyhow!("lethal hit should record once"));
	}
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 7.0,
		point: Vec3::ZERO,
	});
	app.update();
	let hist = history(&app)?;
	if hist.len() != 1 {
		return Err(anyhow!("already-dead hits are rejected at apply and must not append"));
	}
	if hist.received_positive_damage_since(DamageActorId::from_entity(victim), 0.0)
		&& hist.records().any(|record| record.requested == 7.0)
	{
		return Err(anyhow!("corpse hit must not look like a new apply"));
	}
	Ok(())
}

#[test]
fn zero_damage_is_recorded_as_zero_effect() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 0.0,
		point: Vec3::ZERO,
	});
	app.update();
	let record = history(&app)?
		.for_recipient(DamageActorId::from_entity(victim))
		.next()
		.copied()
		.ok_or_else(|| anyhow!("zero-amount hit on a live target should still be recorded"))?;
	if !record.is_zero_effect() || record.is_positive() || record.applied != 0.0 {
		return Err(anyhow!("zero damage must be distinguishable from a positive apply"));
	}
	if history(&app)?.received_positive_damage_since(DamageActorId::from_entity(victim), 0.0) {
		return Err(anyhow!("healing lookback must ignore zero-effect hits"));
	}
	Ok(())
}

#[test]
fn victim_retirement_leaves_history_queryable() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(10.0)).id();
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 10.0,
		point: Vec3::new(1.0, 2.0, 3.0),
	});
	app.update();
	let retired = DamageActorId::from_entity(victim);
	app.world_mut().entity_mut(victim).despawn();
	app.update();
	let record = history(&app)?
		.for_recipient(retired)
		.next()
		.copied()
		.ok_or_else(|| anyhow!("history must outlive the retired victim"))?;
	if !record.is_positive() || record.applied != 10.0 {
		return Err(anyhow!("retired victim record should keep the applied HP"));
	}
	Ok(())
}

#[test]
fn history_set_runs_after_apply() -> Result<()> {
	let mut app = history_app();
	let victim = app.world_mut().spawn(Health::from_max(100.0)).id();
	app.world_mut().write_message(Hit {
		target: victim,
		source: None,
		amount: 7.0,
		point: Vec3::ZERO,
	});
	app.update();
	if !history(&app)?.received_positive_damage_since(DamageActorId::from_entity(victim), 0.0) {
		return Err(anyhow!("DamageSystems::History should ingest the same-frame apply"));
	}
	Ok(())
}

#[test]
fn maintain_respects_policy_on_the_clock() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.init_resource::<DamageHistory>()
		.insert_resource(DamageHistoryPolicy { retain_secs: 1.0, max_records: 8 })
		.add_message::<DamageApplied>()
		.add_message::<ClearDamageHistory>()
		.add_systems(PostUpdate, maintain_damage_history.in_set(DamageSystems::History));
	let victim = app.world_mut().spawn_empty().id();
	set_elapsed(&mut app, 1.0);
	write_applied(&mut app, victim, None, 2.0);
	app.update();
	set_elapsed(&mut app, 3.0);
	app.update();
	if !history(&app)?.is_empty() {
		return Err(anyhow!("records older than retain_secs should expire"));
	}
	Ok(())
}
