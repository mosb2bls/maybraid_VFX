//! Validate the complete Self Lubikan loop in the world: points → charge → activation → healing.

use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::{DamageApplied, Health, Hit};
use lubikans::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationPhase,
	LubikanActivationRequest, LubikanActivationSession, LubikanChargeChanged,
	LubikanChargeReason, LubikanCombatAward, LubikanLedger, LubikanOwner, LubikanOwnerLifetime,
	LubikanOwnerLifetimeKind, LubikanSelfHeal,
};
use maybraid_character_controller::CharacterIntent;
use maybraid_world::{WorldGameplayEnabled, WorldPlugin};
use player::{Player, PlayerPlugin};
use std::time::Duration;

const DT: f32 = 0.1;

fn self_loop_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
		.add_plugins(PlayerPlugin)
		.add_plugins(WorldPlugin::game());
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(10));
	app
}

fn spawn_player(app: &mut App) -> Entity {
	let player = app
		.world_mut()
		.spawn((
			Player,
			Health { current: 40.0, max: 100.0 },
			Name::new("Test Player"),
		))
		.id();
	app.update();
	player
}

fn spawn_npc(app: &mut App) -> Entity {
	app.world_mut()
		.spawn((
			Health { current: 30.0, max: 30.0 },
			GlobalTransform::default(),
			Name::new("Test NPC"),
		))
		.id()
}

fn hit_npc(app: &mut App, player: Entity, npc: Entity, damage: f32, remaining: f32) {
	app.world_mut().write_message(Hit {
		target: npc,
		source: Some(player),
		amount: damage,
		point: Vec3::ZERO,
	});
	// Update health manually for test
	if let Some(mut health) = app.world_mut().get_mut::<Health>(npc) {
		health.current = remaining;
	}
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

fn activate_self(app: &mut App, player: Entity) {
	app.world_mut()
		.write_message(CharacterIntent::PowerUseItem);
	app.update();
}

fn drain_activation_finished(app: &mut App) -> Vec<LubikanActivationFinished> {
	app.world_mut()
		.resource_mut::<Messages<LubikanActivationFinished>>()
		.drain()
		.collect()
}

#[test]
fn earn_charge_activate_heal_finish_and_reactivate() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app);
	
	// Verify player is a Lubikan owner with Self ledger
	anyhow::ensure!(
		app.world().get::<LubikanOwner>(player).is_some(),
		"player should have LubikanOwner"
	);
	anyhow::ensure!(
		app.world().get::<LubikanLedger>(player).is_some(),
		"player should have LubikanLedger"
	);
	
	// Get activation cost
	let cost = app
		.world()
		.resource::<lubikans::LubikanCatalog>()
		.self_definition()
		.activation_cost;
	
	// Earn enough points for 25+ charge through combat
	// Default award: hit=1.0, head=2.0, kill=5.0 (stacked)
	let npc1 = spawn_npc(&mut app);
	let npc2 = spawn_npc(&mut app);
	let npc3 = spawn_npc(&mut app);
	let npc4 = spawn_npc(&mut app);
	
	// Hit NPC1 (1 charge)
	hit_npc(&mut app, player, npc1, 10.0, 20.0);
	app.update();
	anyhow::ensure!(
		charge(&app, player) >= 1.0,
		"hit should award 1 charge, got {}",
		charge(&app, player)
	);
	
	// Kill NPC1 (hit 1 + kill 5 = 6 total charge, 7 cumulative)
	hit_npc(&mut app, player, npc1, 30.0, 0.0);
	app.update();
	anyhow::ensure!(
		charge(&app, player) >= 6.0,
		"kill should award 6 charge (stacked), got {}",
		charge(&app, player)
	);
	
	// Hit and kill NPC2 (7 cumulative)
	hit_npc(&mut app, player, npc2, 30.0, 0.0);
	app.update();
	
	// Hit and kill NPC3 (7 cumulative)
	hit_npc(&mut app, player, npc3, 30.0, 0.0);
	app.update();
	
	// Hit and kill NPC4 (7 cumulative)
	hit_npc(&mut app, player, npc4, 30.0, 0.0);
	app.update();
	
	let earned = charge(&app, player);
	anyhow::ensure!(
		earned >= 25.0,
		"should have 25+ charge, got {earned}"
	);
	
	// Take damage to enable healing cap at 70%
	{
		let mut health = app
			.world_mut()
			.get_mut::<Health>(player)
			.ok_or_else(|| anyhow::anyhow!("health"))?;
		health.take_damage(20.0);
	}
	anyhow::ensure!(
		health_current(&app, player) == 20.0,
		"player should be at 20 HP after damage"
	);
	
	// Activate Self
	activate_self(&mut app, player);
	
	// Verify activation was accepted
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should be active"
	);
	
	// Verify charge was spent
	let after_activation = charge(&app, player);
	anyhow::ensure!(
		after_activation < earned - cost + 1.0,
		"charge should be spent, earned {earned}, cost {cost}, remaining {after_activation}"
	);
	
	// Verify healing channel is active
	let heal = app.world().resource::<LubikanSelfHeal>();
	anyhow::ensure!(heal.active, "heal should be active");
	anyhow::ensure!(heal.owner == Some(player), "heal owner should be player");
	
	// Let healing run (5% per second, so 5 HP per second for max 100)
	let start_hp = health_current(&app, player);
	for _ in 0..50 {
		app.update();
		let current = health_current(&app, player);
		if current >= 70.0 {
			break;
		}
	}
	
	// Verify healing reached 70% cap
	let healed_hp = health_current(&app, player);
	anyhow::ensure!(
		(healed_hp - 70.0).abs() < 1.0,
		"should heal to 70% cap, got {healed_hp}"
	);
	anyhow::ensure!(
		healed_hp > start_hp,
		"should have healed from {start_hp} to {healed_hp}"
	);
	
	// Verify completion flash
	let finished = drain_activation_finished(&mut app);
	anyhow::ensure!(!finished.is_empty(), "should emit LubikanActivationFinished");
	anyhow::ensure!(
		finished[0].outcome == LubikanActivationOutcome::Completed,
		"should complete successfully"
	);
	
	// Verify return to Idle
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Idle,
		"should return to Idle after completion"
	);
	anyhow::ensure!(
		!app.world().resource::<LubikanSelfHeal>().active,
		"heal channel should be inactive"
	);
	
	// Earn more charge and activate again
	let npc5 = spawn_npc(&mut app);
	hit_npc(&mut app, player, npc5, 30.0, 0.0);
	app.update();
	
	let second_charge = charge(&app, player);
	anyhow::ensure!(
		second_charge >= cost,
		"should have enough charge for second activation"
	);
	
	// Take more damage
	{
		let mut health = app
			.world_mut()
			.get_mut::<Health>(player)
			.ok_or_else(|| anyhow::anyhow!("health"))?;
		health.take_damage(15.0);
	}
	
	// Activate again
	activate_self(&mut app, player);
	
	// Verify second activation works
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"second activation should be active"
	);
	anyhow::ensure!(
		app.world().resource::<LubikanSelfHeal>().active,
		"second heal should be active"
	);
	
	Ok(())
}

#[test]
fn pause_freezes_heal_and_ledger() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app);
	
	// Get activation cost and fund the player
	let cost = app
		.world()
		.resource::<lubikans::LubikanCatalog>()
		.self_definition()
		.activation_cost;
	
	{
		let mut ledger = app
			.world_mut()
			.get_mut::<LubikanLedger>(player)
			.ok_or_else(|| anyhow::anyhow!("ledger"))?;
		ledger.apply_to_equipped(cost);
	}
	
	// Take damage
	{
		let mut health = app
			.world_mut()
			.get_mut::<Health>(player)
			.ok_or_else(|| anyhow::anyhow!("health"))?;
		health.take_damage(30.0);
	}
	
	// Activate
	activate_self(&mut app, player);
	app.update();
	
	let hp_before_pause = health_current(&app, player);
	
	// Pause (disable gameplay)
	app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = false;
	app.update();
	app.update();
	app.update();
	
	// Verify healing is frozen
	let hp_during_pause = health_current(&app, player);
	anyhow::ensure!(
		(hp_during_pause - hp_before_pause).abs() < 1.0,
		"health should not change during pause"
	);
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should remain active during pause"
	);
	
	// Resume
	app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = true;
	app.update();
	app.update();
	app.update();
	
	// Verify healing resumed
	let hp_after_resume = health_current(&app, player);
	anyhow::ensure!(
		hp_after_resume > hp_during_pause,
		"healing should resume after unpause"
	);
	
	Ok(())
}

#[test]
fn session_restart_resets_charge() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app);
	
	// Earn charge
	let npc = spawn_npc(&mut app);
	hit_npc(&mut app, player, npc, 30.0, 0.0);
	app.update();
	
	let earned = charge(&app, player);
	anyhow::ensure!(earned > 0.0, "should have earned charge");
	
	// Send session restart signal
	app.world_mut().write_message(LubikanOwnerLifetime::new(
		None,
		None,
		LubikanOwnerLifetimeKind::RestartSession,
	));
	app.update();
	
	// Verify charge is reset
	let after_restart = charge(&app, player);
	anyhow::ensure!(
		after_restart == 0.0,
		"charge should reset to 0 after session restart, got {after_restart}"
	);
	
	Ok(())
}
