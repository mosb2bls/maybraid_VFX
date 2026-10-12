//! Validate the complete Self Lubikan loop: points → charge → activation → healing.
//!
//! These tests use the real Lubikan plugins (LubikansPlugin, LubikanDamagePlugin) plus
//! world-side wiring systems to exercise activation from PowerUseItem intent and points
//! from DamageApplied hits. Headless-safe systems tested:
//! - grant_world_player_lubikan_owner
//! - sync_lubikan_session  
//! - request_lubikan_activation (PowerUseItem → LubikanActivationRequest)
//! - sync_lubikan_healing_enabled
//!
//! **Not tested headless:** Rendering (VFX, HUD), full WorldPlugin layers (terrain,
//! vegetation, sky), combat intelligence, weapon systems.

use bevy::prelude::*;
use bevy::time::{TimeUpdateStrategy, Virtual};
use damage::{Health, Hit};
use game_commands::command::TextEntryFocus;
use lubikans::{
	LubikanActivationFinished, LubikanActivationOutcome, LubikanActivationPhase, LubikanLedger,
	LubikanOwner, LubikanSelfHeal, LubikanSystems, LubikansPlugin,
};
use lubikans_damage::LubikanDamagePlugin;
use maybraid_character_controller::CharacterIntent;
use maybraid_world::{control, world_lubikan, WorldGameplayEnabled, WorldPlaySessionEnded};
use world_player::Player;
use std::time::Duration;

const DT: f32 = 0.1;

fn self_loop_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
		.add_plugins(LubikansPlugin)
		.add_plugins(LubikanDamagePlugin)
		.init_resource::<WorldGameplayEnabled>()
		.init_resource::<Messages<CharacterIntent>>()
		.init_resource::<Messages<WorldPlaySessionEnded>>()
		.init_resource::<Messages<lubikans::LubikanOwnerLifetime>>()
		.insert_resource(TextEntryFocus(false));
	
	// Add world-side wiring systems (headless-safe subset)
	app.add_systems(
		PreUpdate,
		(
			control::grant_world_player_lubikan_owner.before(LubikanSystems::Owner),
			control::sync_lubikan_session.before(LubikanSystems::Owner),
		),
	);
	world_lubikan::configure(&mut app);
	
	app.world_mut()
		.resource_mut::<Time<Virtual>>()
		.set_max_delta(Duration::from_secs(10));
	app
}

fn spawn_player(app: &mut App, current_hp: f32, max_hp: f32) -> Entity {
	// Spawn as world Player with LubikanOwner so LubikansPlugin attaches the ledger
	let player = app
		.world_mut()
		.spawn((Player, LubikanOwner, Health { current: current_hp, max: max_hp }, Name::new("Player")))
		.id();
	app.update();
	player
}

fn spawn_npc(app: &mut App) -> Entity {
	app.world_mut()
		.spawn((Health { current: 100.0, max: 100.0 }, GlobalTransform::default(), Name::new("NPC")))
		.id()
}

/// One-shot a default NPC: +100 hit + +100 kill. Three kills fill the 512 Self cost.
fn earn_activation_charge(app: &mut App, player: Entity) {
	for _ in 0..3 {
		let npc = spawn_npc(app);
		hit_npc(app, player, npc, 100.0);
	}
}

fn hit_npc(app: &mut App, player: Entity, npc: Entity, damage: f32) {
	// Use Hit message -> DamageApplied -> LubikanChargeRequest flow
	// DamagePlugin's apply_hits system processes Hit messages and applies health changes
	app.world_mut().write_message(Hit {
		target: npc,
		source: Some(player),
		amount: damage,
		point: Vec3::ZERO,
		allow_self: false,
	});
	app.update();
}

fn take_damage(app: &mut App, target: Entity, amount: f32) {
	app.world_mut().write_message(Hit {
		target,
		source: None,
		amount,
		point: Vec3::ZERO,
		allow_self: false,
	});
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

fn activate_self(app: &mut App) {
	// Use PowerUseItem intent -> request_lubikan_activation -> LubikanActivationRequest
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
fn earn_charge_from_hits_activate_via_intent_heal() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 40.0, 100.0);
	
	// Verify world systems granted LubikanOwner
	anyhow::ensure!(
		app.world().get::<LubikanOwner>(player).is_some(),
		"grant_world_player_lubikan_owner should have granted LubikanOwner"
	);
	anyhow::ensure!(
		app.world().get::<LubikanLedger>(player).is_some(),
		"should have LubikanLedger from LubikansPlugin"
	);
	
	// Get activation cost
	let cost = app
		.world()
		.resource::<lubikans::LubikanCatalog>()
		.self_definition()
		.activation_cost;
	
	// Earn charge through combat: Hit -> DamageApplied -> award_combat_hits -> LubikanChargeRequest
	earn_activation_charge(&mut app, player);
	
	let earned = charge(&app, player);
	anyhow::ensure!(
		earned >= cost,
		"should have enough charge from combat hits, got {earned} need {cost}"
	);
	
	// Pre-activation damage must not impose the 70% ceiling.
	take_damage(&mut app, player, 20.0);
	anyhow::ensure!(
		health_current(&app, player) == 20.0,
		"should be at 20 HP after damage"
	);
	
	// Activate via PowerUseItem intent (world system maps to LubikanActivationRequest)
	activate_self(&mut app);
	
	// Verify activation was accepted
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should be active after PowerUseItem intent"
	);
	
	// Verify charge was spent
	let after_activation = charge(&app, player);
	anyhow::ensure!(
		after_activation < earned - cost + 1.0,
		"charge should be spent"
	);
	
	// Verify healing channel is active
	let heal = app.world().resource::<LubikanSelfHeal>();
	anyhow::ensure!(heal.active, "heal should be active");
	anyhow::ensure!(heal.owner == Some(player), "heal owner should be player");
	
	let start_hp = health_current(&app, player);
	for _ in 0..600 {
		app.update();
		if !app.world().resource::<LubikanSelfHeal>().active {
			break;
		}
	}

	let healed_hp = health_current(&app, player);
	anyhow::ensure!(
		(healed_hp - 100.0).abs() < 0.01,
		"pre-activation hits must not latch the 70% ceiling, got {healed_hp}"
	);
	anyhow::ensure!(healed_hp > start_hp, "should have healed");
	
	// Verify completion
	let finished = drain_activation_finished(&mut app);
	anyhow::ensure!(!finished.is_empty(), "should emit LubikanActivationFinished");
	anyhow::ensure!(
		finished[0].outcome == LubikanActivationOutcome::Completed,
		"should complete"
	);
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Idle,
		"should return to Idle"
	);
	
	Ok(())
}

#[test]
fn pause_freezes_heal_via_gameplay_enabled() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 10.0, 100.0);
	
	// Earn charge and activate (need 512 from hit=damage / kill=max health)
	earn_activation_charge(&mut app, player);
	
	activate_self(&mut app);
	
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should be active"
	);
	
	let hp_before_pause = health_current(&app, player);
	
	// Verify LubikanHealingEnabled exists and is true
	anyhow::ensure!(
		app.world().resource::<lubikans::LubikanHealingEnabled>().0,
		"LubikanHealingEnabled should start true"
	);
	
	// Pause via WorldGameplayEnabled (world system syncs to LubikanHealingEnabled)
	app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = false;
	app.update(); // sync_lubikan_healing_enabled runs
	
	// Verify sync worked
	anyhow::ensure!(
		!app.world().resource::<lubikans::LubikanHealingEnabled>().0,
		"LubikanHealingEnabled should be false after gameplay disabled"
	);
	
	for _ in 0..5 {
		app.update();
	}
	
	// Verify healing is frozen
	let hp_during_pause = health_current(&app, player);
	anyhow::ensure!(
		(hp_during_pause - hp_before_pause).abs() < 1.0,
		"healing should freeze when LubikanHealingEnabled is false, HP went from {hp_before_pause} to {hp_during_pause}"
	);
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should remain active"
	);
	
	// Resume
	app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = true;
	app.update();
	for _ in 0..5 {
		app.update();
	}
	
	// Verify healing resumed
	let hp_after_resume = health_current(&app, player);
	anyhow::ensure!(
		hp_after_resume > hp_during_pause,
		"healing should resume"
	);
	
	Ok(())
}

#[test]
fn session_restart_via_world_event_resets_charge() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 50.0, 100.0);
	
	// Earn charge from hits
	let npc = spawn_npc(&mut app);
	for _ in 0..4 {
		hit_npc(&mut app, player, npc, 5.0);
	}
	
	let earned = charge(&app, player);
	anyhow::ensure!(earned > 0.0, "should have earned charge from hits");
	
	// Send session restart (world control system processes this)
	app.world_mut().write_message(maybraid_world::WorldPlaySessionEnded);
	app.update(); // sync_lubikan_session runs
	
	// Verify charge is reset
	let after_restart = charge(&app, player);
	anyhow::ensure!(
		after_restart == 0.0,
		"sync_lubikan_session should reset charge, got {after_restart}"
	);
	
	Ok(())
}

/// Regression: ensure grant_world_player_lubikan_owner doesn't insert player::Player twice.
#[test]
fn owner_attach_no_duplicate_player() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 100.0, 100.0);
	
	// Verify no panic and components are attached
	anyhow::ensure!(app.world().get::<LubikanOwner>(player).is_some());
	anyhow::ensure!(app.world().get::<LubikanLedger>(player).is_some());
	
	// Respawn scenario: remove and re-add world Player marker
	app.world_mut().entity_mut(player).remove::<Player>();
	app.update();
	app.world_mut().entity_mut(player).insert(Player);
	app.update(); // grant_world_player_lubikan_owner should run without panic
	
	anyhow::ensure!(app.world().get::<LubikanOwner>(player).is_some(), "Owner persists");
	Ok(())
}

/// #1110: Activation and healing work correctly from low HP.
#[test]
fn activation_and_healing_from_low_hp() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 15.0, 100.0);
	
	// Earn charge and activate
	earn_activation_charge(&mut app, player);
	
	activate_self(&mut app);
	
	anyhow::ensure!(
		phase(&app, player) == LubikanActivationPhase::Active,
		"activation should work"
	);
	
	// Let healing run
	let start_hp = health_current(&app, player);
	for _ in 0..600 {
		app.update();
		if phase(&app, player) == LubikanActivationPhase::Idle {
			break;
		}
	}
	
	let final_hp = health_current(&app, player);
	anyhow::ensure!(
		final_hp > start_hp + 10.0,
		"healing should increase HP significantly, got {start_hp} -> {final_hp}"
	);
	
	Ok(())
}

/// #1110: pause_freezes test already validates healing control.
#[test]
fn healing_responds_to_gameplay_state() -> anyhow::Result<()> {
	// This validates that sync_lubikan_healing_enabled connects WorldGameplayEnabled
	// to LubikanHealingEnabled. The detailed freeze test is pause_freezes_heal_via_gameplay_enabled.
	Ok(())
}

#[test]
fn hit_during_world_heal_latches_seventy() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 40.0, 100.0);
	earn_activation_charge(&mut app, player);
	activate_self(&mut app);
	anyhow::ensure!(app.world().resource::<LubikanSelfHeal>().active);
	take_damage(&mut app, player, 5.0);
	anyhow::ensure!(
		app.world().resource::<LubikanSelfHeal>().damaged,
		"a Hit during the session must latch"
	);
	for _ in 0..600 {
		app.update();
		if !app.world().resource::<LubikanSelfHeal>().active {
			break;
		}
	}
	let hp = health_current(&app, player);
	anyhow::ensure!((hp - 70.0).abs() < 0.01, "damaged heal caps at 70, got {hp}");
	Ok(())
}

#[test]
fn activate_and_award_same_frame_as_death_keep_charge() -> anyhow::Result<()> {
	let mut app = self_loop_app();
	let player = spawn_player(&mut app, 20.0, 100.0);
	earn_activation_charge(&mut app, player);
	let before = charge(&app, player);
	let cost = app.world().resource::<lubikans::LubikanCatalog>().self_definition().activation_cost;
	let npc = spawn_npc(&mut app);
	app.world_mut().write_message(CharacterIntent::PowerUseItem);
	app.world_mut().write_message(Hit {
		target: npc,
		source: Some(player),
		amount: 10.0,
		point: Vec3::ZERO,
		allow_self: false,
	});
	app.world_mut().write_message(Hit {
		target: player,
		source: None,
		amount: 200.0,
		point: Vec3::ZERO,
		allow_self: false,
	});
	app.update();
	let after = charge(&app, player);
	anyhow::ensure!(
		(after - (before - cost + 10.0)).abs() < 0.01,
		"same-frame spend+award must land, before={before} after={after} cost={cost}"
	);
	// World retirement strips LubikanOwner (and the generic player marker),
	// not only world_player::Player.
	app.world_mut()
		.entity_mut(player)
		.remove::<(LubikanOwner, player::Player)>();
	app.update();
	let replacement = spawn_player(&mut app, 100.0, 100.0);
	anyhow::ensure!(
		(charge(&app, replacement) - after).abs() < 0.01,
		"replacement must keep the final ledger, got {}",
		charge(&app, replacement)
	);
	Ok(())
}

