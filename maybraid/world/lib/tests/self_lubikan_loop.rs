//! Validate the complete Self Lubikan loop: points → charge → activation → healing.
//!
//! These tests use the real Lubikan plugins (LubikansPlugin, LubikanDamagePlugin) and
//! verify charge flows through LubikanChargeRequest → LubikanSystems::Award, not direct
//! ledger writes. The world control systems (owner granting, session sync, activation
//! request mapping) are tested implicitly through the lubikans plugin tests and the
//! full world app manual testing.

use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::{DamageApplied, Health};
use lubikans::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationPhase,
	LubikanActivationRequest, LubikanChargeReason, LubikanChargeRequest, LubikanLedger,
	LubikanOwner, LubikanOwnerLifetime, LubikanOwnerLifetimeKind, LubikanSelfHeal, LubikansPlugin,
};
use lubikans_damage::LubikanDamagePlugin;
use std::time::Duration;

const DT: f32 = 0.1;

fn self_loop_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
		.add_plugins(LubikansPlugin)
		.add_plugins(LubikanDamagePlugin);
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(10));
	app
}

fn spawn_owner(app: &mut App, current_hp: f32, max_hp: f32) -> Entity {
	let owner = app
		.world_mut()
		.spawn((LubikanOwner, Health { current: current_hp, max: max_hp }, Name::new("Owner")))
		.id();
	app.update();
	owner
}

fn award_charge_via_request(app: &mut App, owner: Entity, amount: f32) {
	// Go through LubikanChargeRequest -> Award path, not direct ledger write
	app.world_mut().write_message(LubikanChargeRequest::equipped(
		owner,
		amount,
		LubikanChargeReason::COMBAT_HIT,
		Some(Vec3::ZERO),
	));
	app.update();
}

fn apply_damage_via_event(app: &mut App, target: Entity, amount: f32) {
	// Calculate remaining health first
	let remaining = app
		.world()
		.get::<Health>(target)
		.map(|h| (h.current - amount).max(0.0))
		.unwrap_or(0.0);
	
	// Simulate damage through DamageApplied
	app.world_mut().write_message(DamageApplied {
		target,
		source: None,
		amount,
		applied: amount,
		remaining,
		point: Vec3::ZERO,
	});
	if let Some(mut health) = app.world_mut().get_mut::<Health>(target) {
		health.take_damage(amount);
	}
	app.update();
}

fn charge(app: &App, player: Entity) -> f32 {
	app.world()
		.get::<LubikanLedger>(player)
		.and_then(|ledger| ledger.account(lubikans::LubikanAccountId::SELF))
		.map(|account| account.charge().current())
		.unwrap_or(0.0)
}

fn phase(app: &App, player: Entity) -> LubikanActivationPhase {
	app.world()
		.get::<LubikanLedger>(player)
		.and_then(|ledger| ledger.account(lubikans::LubikanAccountId::SELF))
		.map(|account| account.activation().phase)
		.unwrap_or(LubikanActivationPhase::Idle)
}

fn health_current(app: &App, player: Entity) -> f32 {
	app.world()
		.get::<Health>(player)
		.map(|health| health.current)
		.unwrap_or(0.0)
}

fn activate_self(app: &mut App, owner: Entity) {
	app.world_mut()
		.write_message(LubikanActivationRequest::equipped(owner));
	app.update();
}

fn drain_activation_finished(app: &mut App) -> Vec<LubikanActivationFinished> {
	app.world_mut()
		.resource_mut::<Messages<LubikanActivationFinished>>()
		.drain()
		.collect()
}

#[test]
fn earn_charge_via_request_activate_heal_finish() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let owner = spawn_owner(&mut app, 40.0, 100.0);
	
	// Verify owner has Self ledger
	anyhow::ensure!(
		app.world().get::<LubikanOwner>(owner).is_some(),
		"should have LubikanOwner"
	);
	anyhow::ensure!(
		app.world().get::<LubikanLedger>(owner).is_some(),
		"should have LubikanLedger"
	);
	
	// Get activation cost
	let cost = app
		.world()
		.resource::<lubikans::LubikanCatalog>()
		.self_definition()
		.activation_cost;
	
	// Award charge through LubikanChargeRequest (not direct ledger write)
	award_charge_via_request(&mut app, owner, 30.0);
	
	let earned = charge(&app, owner);
	anyhow::ensure!(
		earned >= 25.0,
		"should have 25+ charge via LubikanChargeRequest, got {earned}"
	);
	
	// Take damage to enable 70% healing cap
	apply_damage_via_event(&mut app, owner, 20.0);
	
	anyhow::ensure!(
		health_current(&app, owner) == 20.0,
		"should be at 20 HP after damage"
	);
	
	// Activate Self
	activate_self(&mut app, owner);
	
	// Verify activation was accepted
	anyhow::ensure!(
		phase(&app, owner) == LubikanActivationPhase::Active,
		"activation should be active"
	);
	
	// Verify charge was spent
	let after_activation = charge(&app, owner);
	anyhow::ensure!(
		after_activation < earned - cost + 1.0,
		"charge should be spent, earned {earned}, cost {cost}, remaining {after_activation}"
	);
	
	// Verify healing channel is active
	let heal = app.world().resource::<LubikanSelfHeal>();
	anyhow::ensure!(heal.active, "heal should be active");
	anyhow::ensure!(heal.owner == Some(owner), "heal owner should match");
	
	// Let healing run (5% per second, so 5 HP per second for max 100)
	// With DT=0.1s per update, healing rate is 0.5 HP per update
	// From 20 HP to 70 HP = 50 HP = 100 updates minimum
	let start_hp = health_current(&app, owner);
	for i in 0..600 {
		app.update();
		let current = health_current(&app, owner);
		// Check if healing finished (stopped at cap or completed)
		if !app.world().resource::<LubikanSelfHeal>().active {
			eprintln!("Healing finished after {} updates at {} HP", i + 1, current);
			break;
		}
	}
	
	// Verify healing reached 70% cap or close
	let healed_hp = health_current(&app, owner);
	anyhow::ensure!(
		healed_hp >= 65.0,
		"should heal near 70% cap, got {healed_hp} from {start_hp}"
	);
	anyhow::ensure!(
		healed_hp > start_hp,
		"should have healed from {start_hp} to {healed_hp}"
	);
	
	// Drain any activation finished messages that occurred
	let finished = drain_activation_finished(&mut app);
	if finished.is_empty() {
		eprintln!("No LubikanActivationFinished at {} HP, phase {:?}", 
			healed_hp, phase(&app, owner));
	}
	anyhow::ensure!(!finished.is_empty(), "should emit LubikanActivationFinished");
	anyhow::ensure!(
		finished[0].outcome == LubikanActivationOutcome::Completed,
		"should complete successfully"
	);
	
	// Verify return to Idle
	anyhow::ensure!(
		phase(&app, owner) == LubikanActivationPhase::Idle,
		"should return to Idle after completion"
	);
	anyhow::ensure!(
		!app.world().resource::<LubikanSelfHeal>().active,
		"heal channel should be inactive"
	);
	
	Ok(())
}

#[test]
fn pause_freezes_heal_not_ledger() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let owner = spawn_owner(&mut app, 10.0, 100.0);
	
	// Get activation cost and award via request
	let cost = app
		.world()
		.resource::<lubikans::LubikanCatalog>()
		.self_definition()
		.activation_cost;
	
	award_charge_via_request(&mut app, owner, cost);
	
	// Activate
	activate_self(&mut app, owner);
	
	let hp_before_pause = health_current(&app, owner);
	
	// Pause healing (LubikanHealingEnabled)
	app.world_mut()
		.resource_mut::<lubikans::LubikanHealingEnabled>()
		.0 = false;
	for _ in 0..5 {
		app.update();
	}
	
	// Verify healing is frozen
	let hp_during_pause = health_current(&app, owner);
	anyhow::ensure!(
		(hp_during_pause - hp_before_pause).abs() < 1.0,
		"health should not change while healing disabled"
	);
	anyhow::ensure!(
		phase(&app, owner) == LubikanActivationPhase::Active,
		"activation should remain active during pause"
	);
	
	// Resume
	app.world_mut()
		.resource_mut::<lubikans::LubikanHealingEnabled>()
		.0 = true;
	for _ in 0..5 {
		app.update();
	}
	
	// Verify healing resumed
	let hp_after_resume = health_current(&app, owner);
	anyhow::ensure!(
		hp_after_resume > hp_during_pause,
		"healing should resume after unpause"
	);
	
	Ok(())
}

#[test]
fn session_restart_resets_charge() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let owner = spawn_owner(&mut app, 50.0, 100.0);
	
	// Award charge via request
	award_charge_via_request(&mut app, owner, 20.0);
	
	let earned = charge(&app, owner);
	anyhow::ensure!(earned > 0.0, "should have earned charge via request");
	
	// Send session restart signal
	app.world_mut().write_message(LubikanOwnerLifetime::new(
		None,
		None,
		LubikanOwnerLifetimeKind::RestartSession,
	));
	app.update();
	
	// Verify charge is reset
	let after_restart = charge(&app, owner);
	anyhow::ensure!(
		after_restart == 0.0,
		"charge should reset to 0 after session restart, got {after_restart}"
	);
	
	Ok(())
}
