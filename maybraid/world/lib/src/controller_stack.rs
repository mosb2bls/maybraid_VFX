//! Stack-tip acceptance on the real production plugin wiring (not chained systems).

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use firearm_user::{apply_fire_intents, FirearmUser};
use firearms::{WeaponFired, WeaponTrigger};
use grenade_user::{apply_throw_intents, GrenadeThrow, GrenadeUser};
use lubikans::{
	LubikanActivationDenied, LubikanActivationRefused, LubikanLedger, LubikanOwner,
	LubikanSelfConfig, LubikansPlugin,
};
use maybraid_character_controller::{
	CharacterControllerPlugin, CharacterIntent, EquippedItemHandler, EquippedItemUse,
	EquippedPowerUse, InventoryCycling, InventoryCyclingCategory,
};
use maybraid_controllers_core::{
	ActorTarget, CaptureKind, InputRouter, InputSource, ScopeOwner, UiTarget,
};
use maybraid_world_controller::{SessionCapabilities, WorldControllerPlugin};
use player::{CameraFollow, Player, PlayerLook, PlayerSystems};
use player_camera::{CameraController, CameraRecoilHandoff, FollowCamera};
use std::time::Duration;
use world_player::Player as VegetationPlayer;

use crate::inventory_cycling::{self, ActiveInventoryCyclingCategory};
use crate::{world_lubikan, WorldGameplayEnabled};
use game_commands::command::TextEntryFocus;

fn stack_app() -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(bevy::input::InputPlugin)
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(1.0 / 60.0)))
		.add_plugins(CharacterControllerPlugin)
		.add_plugins(WorldControllerPlugin)
		.add_plugins(LubikansPlugin)
		.add_message::<WeaponFired>()
		.add_message::<CameraRecoilHandoff>()
		.configure_sets(
			Update,
			PlayerSystems::Intent.after(maybraid_character_controller::CharacterControlSystems),
		)
		.add_systems(
			Update,
			(apply_fire_intents, apply_throw_intents).in_set(PlayerSystems::Intent),
		)
		.insert_resource(WorldGameplayEnabled(true))
		.insert_resource(TextEntryFocus(false));
	inventory_cycling::configure(&mut app);
	world_lubikan::configure(&mut app);
	app
}

fn assign(app: &mut App, actor: Entity) {
	app.world_mut()
		.resource_mut::<InputRouter>()
		.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
}

fn no_thrown_grenade(app: &mut App) -> bool {
	let world = app.world_mut();
	world.query::<&grenades::ThrownGrenade>().iter(world).next().is_none()
}

#[test]
fn two_actors_only_the_delivered_handler_fires() {
	let mut app = stack_app();
	let gun_a = app.world_mut().spawn(WeaponTrigger(false)).id();
	let gun_b = app.world_mut().spawn(WeaponTrigger(false)).id();
	let a = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			FirearmUser::holding(gun_a),
			EquippedItemHandler::Firearm,
			InventoryCycling::default(),
		))
		.id();
	let b = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			FirearmUser::holding(gun_b),
			EquippedItemHandler::Firearm,
			InventoryCycling::default(),
		))
		.id();
	assign(&mut app, a);
	app.world_mut().write_message(EquippedItemUse::new(
		ActorTarget::new(a),
		EquippedItemHandler::Firearm,
		1.0,
		true,
	));
	app.update();
	assert_eq!(app.world().get::<WeaponTrigger>(gun_a).map(|t| t.0), Some(true));
	assert_eq!(app.world().get::<WeaponTrigger>(gun_b).map(|t| t.0), Some(false));
	let _ = (a, b);
}

#[test]
fn two_actors_in_different_item_modes() {
	let mut app = stack_app();
	let gun = app.world_mut().spawn(WeaponTrigger(false)).id();
	let held = app.world_mut().spawn_empty().id();
	let bag = app
		.world_mut()
		.spawn(Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		})
		.id();
	let shooter = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			FirearmUser::holding(gun),
			EquippedItemHandler::Firearm,
			InventoryCycling::default(),
		))
		.id();
	let thrower = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			InventoryUser::carrying(bag),
			GrenadeUser::holding(held),
			GrenadeThrow::default(),
			EquippedItemHandler::Grenade,
			InventoryCycling::default(),
		))
		.id();
	assign(&mut app, shooter);
	app.world_mut().write_message(EquippedItemUse::new(
		ActorTarget::new(shooter),
		EquippedItemHandler::Firearm,
		1.0,
		true,
	));
	app.update();
	assert_eq!(app.world().get::<WeaponTrigger>(gun).map(|t| t.0), Some(true));
	assert!(!app.world().get::<GrenadeThrow>(thrower).is_some_and(GrenadeThrow::cooking));

	assign(&mut app, thrower);
	app.world_mut().write_message(CharacterIntent::UseItem(1.0));
	app.update();
	assert!(app.world().get::<GrenadeThrow>(thrower).is_some_and(GrenadeThrow::cooking));
}

#[test]
fn grenade_user_does_not_fire_a_firearm() {
	let mut app = stack_app();
	let gun = app.world_mut().spawn(WeaponTrigger(false)).id();
	let held = app.world_mut().spawn_empty().id();
	let bag = app
		.world_mut()
		.spawn(Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		})
		.id();
	let actor = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			InventoryUser::carrying(bag),
			FirearmUser::holding(gun),
			GrenadeUser::holding(held),
			GrenadeThrow::default(),
			EquippedItemHandler::Grenade,
			InventoryCycling::default(),
		))
		.id();
	assign(&mut app, actor);
	app.world_mut().write_message(CharacterIntent::UseItem(1.0));
	app.update();
	assert_eq!(app.world().get::<WeaponTrigger>(gun).map(|t| t.0), Some(false));
	assert!(matches!(
		app.world().get::<GrenadeThrow>(actor).map(|throw| throw.phase),
		Some(grenade_user::GrenadePhase::Cooking { .. })
	));
}

#[test]
fn denied_shot_produces_no_recoil() {
	let mut app = stack_app();
	let camera = app
		.world_mut()
		.spawn((Camera3d::default(), FollowCamera::default(), CameraController::default()))
		.id();
	let gun = app.world_mut().spawn(WeaponTrigger(false)).id();
	let shooter = app
		.world_mut()
		.spawn((
			Player,
			CameraFollow,
			FirearmUser::holding(gun),
			PlayerLook::default(),
			EquippedItemHandler::Firearm,
		))
		.id();
	assign(&mut app, shooter);
	app.world_mut().write_message(EquippedItemUse::new(
		ActorTarget::new(shooter),
		EquippedItemHandler::Firearm,
		1.0,
		true,
	));
	app.update();
	assert_eq!(app.world().get::<WeaponTrigger>(gun).map(|t| t.0), Some(true));
	assert!(app.world_mut().resource_mut::<Messages<WeaponFired>>().drain().next().is_none());
	assert!(app
		.world_mut()
		.resource_mut::<Messages<CameraRecoilHandoff>>()
		.drain()
		.next()
		.is_none());
	let camera = app.world().entity(camera).get::<CameraController>().unwrap();
	assert_eq!(camera.yaw, 0.0);
	assert_eq!(camera.pitch, 0.0);
}

#[test]
fn capture_does_not_release_a_cooking_grenade() {
	let mut app = stack_app();
	let held = app.world_mut().spawn_empty().id();
	let bag = app
		.world_mut()
		.spawn(Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		})
		.id();
	let actor = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			InventoryUser::carrying(bag),
			GrenadeUser::holding(held),
			GrenadeThrow::default(),
			EquippedItemHandler::Grenade,
		))
		.id();
	assign(&mut app, actor);
	app.world_mut().write_message(CharacterIntent::UseItem(1.0));
	app.update();
	assert!(app.world().get::<GrenadeThrow>(actor).is_some_and(|throw| throw.cooking()));
	app.world_mut().resource_mut::<InputRouter>().acquire_capture(
		InputSource::local_pad(),
		ScopeOwner::ui(UiTarget::new(Entity::from_bits(99))),
		CaptureKind::Overlay,
	);
	app.update();
	assert!(
		app.world().get::<GrenadeThrow>(actor).is_some_and(|throw| throw.cooking()),
		"input suppression must not release"
	);
	assert!(no_thrown_grenade(&mut app));
}

#[test]
fn reassignment_mid_cook_freezes_the_old_actor() {
	let mut app = stack_app();
	let held = app.world_mut().spawn_empty().id();
	let bag = app
		.world_mut()
		.spawn(Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		})
		.id();
	let cook = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			InventoryUser::carrying(bag),
			GrenadeUser::holding(held),
			GrenadeThrow::default(),
			EquippedItemHandler::Grenade,
		))
		.id();
	let other = app
		.world_mut()
		.spawn((Player, VegetationPlayer, EquippedItemHandler::Firearm, InventoryCycling::default()))
		.id();
	assign(&mut app, cook);
	app.world_mut().write_message(CharacterIntent::UseItem(1.0));
	app.update();
	assert!(app.world().get::<GrenadeThrow>(cook).is_some_and(GrenadeThrow::cooking));
	assign(&mut app, other);
	app.update();
	assert!(
		app.world().get::<GrenadeThrow>(cook).is_some_and(GrenadeThrow::cooking),
		"reassignment must cancel delivery without releasing the cook"
	);
	assert!(no_thrown_grenade(&mut app));
}

#[test]
fn category_then_cycle_in_one_plugin_frame() {
	use character_items::{SkillMapKind, SkillMapSpec};

	let mut app = stack_app();
	let inventory = Inventory {
		items: vec![
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
		],
		clothing: Vec::new(),
		weapons: Vec::new(),
		skills: vec![0, 1],
		tanzes: Vec::new(),
	};
	let bag = app.world_mut().spawn(inventory).id();
	let actor = app
		.world_mut()
		.spawn((VegetationPlayer, InventoryUser::carrying(bag), InventoryCycling::default()))
		.id();
	assign(&mut app, actor);
	app.world_mut().write_message(CharacterIntent::SelectInventoryCyclingCategory(
		InventoryCyclingCategory::SkillMaps,
	));
	app.world_mut().write_message(CharacterIntent::SwapActive);
	app.update();
	let bag = app.world().get::<Inventory>(bag).expect("bag");
	assert_eq!(
		bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
		Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
	);
	assert_eq!(
		app.world().resource::<ActiveInventoryCyclingCategory>().0,
		InventoryCyclingCategory::SkillMaps
	);
}

#[test]
fn lubikan_spends_once_from_addressed_dispatch() {
	let mut app = stack_app();
	let actor = app
		.world_mut()
		.spawn((
			VegetationPlayer,
			LubikanOwner,
			LubikanLedger::with_self(&LubikanSelfConfig::default()),
		))
		.id();
	assign(&mut app, actor);
	{
		let mut ledger = app.world_mut().get_mut::<LubikanLedger>(actor).unwrap();
		let _ = ledger.apply_to_equipped(512.0);
	}
	let before = app
		.world()
		.get::<LubikanLedger>(actor)
		.unwrap()
		.account(lubikans::LubikanAccountId::SELF)
		.unwrap()
		.charge()
		.current();
	app.world_mut().write_message(EquippedPowerUse::new(ActorTarget::new(actor)));
	app.update();
	let after = app
		.world()
		.get::<LubikanLedger>(actor)
		.unwrap()
		.account(lubikans::LubikanAccountId::SELF)
		.unwrap()
		.charge()
		.current();
	assert!(after < before, "first spend must reduce charge");
	app.world_mut().write_message(EquippedPowerUse::new(ActorTarget::new(actor)));
	app.update();
	let refused = app
		.world_mut()
		.resource_mut::<Messages<LubikanActivationRefused>>()
		.drain()
		.next();
	assert!(
		refused.is_some_and(|refused| refused.reason == LubikanActivationDenied::AlreadyActive),
		"second spend must refuse, not charge again"
	);
	let again = app
		.world()
		.get::<LubikanLedger>(actor)
		.unwrap()
		.account(lubikans::LubikanAccountId::SELF)
		.unwrap()
		.charge()
		.current();
	assert_eq!(again, after);
}

#[test]
fn nested_captures_release_independently_on_the_router() {
	let mut app = stack_app();
	let actor = app.world_mut().spawn((Player, EquippedItemHandler::Firearm)).id();
	assign(&mut app, actor);
	let overlay = app.world_mut().resource_mut::<InputRouter>().acquire_capture(
		InputSource::local_pad(),
		ScopeOwner::ui(UiTarget::new(Entity::from_bits(2))),
		CaptureKind::Overlay,
	);
	let menu = app.world_mut().resource_mut::<InputRouter>().acquire_capture(
		InputSource::local_pad(),
		ScopeOwner::ui(UiTarget::new(Entity::from_bits(3))),
		CaptureKind::MenuNav,
	);
	assert!(app.world().resource::<InputRouter>().blocks_actor(InputSource::local_pad()));
	app.world_mut().resource_mut::<InputRouter>().release_capture(menu);
	assert!(
		app.world().resource::<InputRouter>().blocks_actor(InputSource::local_pad()),
		"releasing the lower menu must not release the overlay"
	);
	app.world_mut().resource_mut::<InputRouter>().release_capture(overlay);
	assert!(!app.world().resource::<InputRouter>().blocks_actor(InputSource::local_pad()));
}

#[test]
fn disconnect_while_holding_clears_delivery_not_the_cook() {
	let mut app = stack_app();
	let held = app.world_mut().spawn_empty().id();
	let bag = app
		.world_mut()
		.spawn(Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		})
		.id();
	let actor = app
		.world_mut()
		.spawn((
			Player,
			VegetationPlayer,
			InventoryUser::carrying(bag),
			GrenadeUser::holding(held),
			GrenadeThrow::default(),
			EquippedItemHandler::Grenade,
		))
		.id();
	assign(&mut app, actor);
	app.world_mut().write_message(CharacterIntent::UseItem(1.0));
	app.update();
	assert!(app.world().get::<GrenadeThrow>(actor).is_some_and(|throw| throw.use_latched));
	app.world_mut().resource_mut::<InputRouter>().disconnect(InputSource::local_pad());
	app.update();
	assert!(
		app.world().get::<GrenadeThrow>(actor).is_some_and(|throw| throw.cooking()),
		"disconnect must cancel delivery without throwing"
	);
	assert!(no_thrown_grenade(&mut app));
}

#[test]
fn same_frame_pause_blocks_inventory_cycle() {
	use character_items::{SkillMapKind, SkillMapSpec};

	let mut app = stack_app();
	let inventory = Inventory {
		items: vec![
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
			InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
		],
		clothing: Vec::new(),
		weapons: Vec::new(),
		skills: vec![0, 1],
		tanzes: Vec::new(),
	};
	let bag = app.world_mut().spawn(inventory).id();
	let actor = app
		.world_mut()
		.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			InventoryCycling::new(InventoryCyclingCategory::SkillMaps),
		))
		.id();
	assign(&mut app, actor);
	app.world_mut().insert_resource(WorldGameplayEnabled(false));
	app.world_mut().resource_mut::<InputRouter>().set_paused(true);
	app.world_mut().write_message(CharacterIntent::SwapActive);
	app.update();
	let bag = app.world().get::<Inventory>(bag).expect("bag");
	assert_eq!(
		bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
		Some(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
		"paused gameplay must not cycle the equipped queue"
	);
}

#[test]
fn loading_play_inventory_resume_home_on_plugin_session() {
	let mut app = stack_app();
	{
		let mut session = app.world_mut().resource_mut::<SessionCapabilities>();
		session.install_loading(true);
	}
	app.update();
	assert!(!app.world().resource::<SessionCapabilities>().gameplay_enabled(
		app.world().resource::<InputRouter>()
	));
	assert!(app.world().resource::<SessionCapabilities>().streaming_enabled());

	{
		let mut session = app.world_mut().resource_mut::<SessionCapabilities>();
		session.install_play(true);
	}
	app.update();
	assert!(app.world().resource::<SessionCapabilities>().gameplay_enabled(
		app.world().resource::<InputRouter>()
	));
	assert!(app.world().resource::<SessionCapabilities>().physics_installed());

	app.world_mut().resource_mut::<InputRouter>().set_paused(true);
	app.update();
	assert!(!app.world().resource::<SessionCapabilities>().gameplay_enabled(
		app.world().resource::<InputRouter>()
	));
	assert!(app.world().resource::<SessionCapabilities>().physics_installed());
	assert!(app.world().resource::<SessionCapabilities>().simulation_active());

	app.world_mut().resource_mut::<InputRouter>().set_paused(false);
	app.update();
	assert!(app.world().resource::<SessionCapabilities>().gameplay_enabled(
		app.world().resource::<InputRouter>()
	));

	{
		let mut session = app.world_mut().resource_mut::<SessionCapabilities>();
		session.uninstall_play();
	}
	app.world_mut().resource_mut::<InputRouter>().set_paused(false);
	app.update();
	assert!(!app.world().resource::<SessionCapabilities>().gameplay_enabled(
		app.world().resource::<InputRouter>()
	));
	assert!(!app.world().resource::<SessionCapabilities>().physics_installed());
	assert!(app.world().resource::<SessionCapabilities>().menu_shell_active());
}

#[test]
fn npc_firearm_does_not_consume_player_dispatch() {
	let mut app = stack_app();
	let gun = app.world_mut().spawn(WeaponTrigger(false)).id();
	let npc = app
		.world_mut()
		.spawn((FirearmUser::holding(gun), EquippedItemHandler::Firearm))
		.id();
	assign(&mut app, npc);
	app.world_mut().write_message(EquippedItemUse::new(
		ActorTarget::new(npc),
		EquippedItemHandler::Firearm,
		1.0,
		true,
	));
	app.update();
	assert_eq!(
		app.world().get::<WeaponTrigger>(gun).map(|t| t.0),
		Some(false),
		"NPC fire stays on intelligence WeaponTrigger, not player EquippedItemUse"
	);
}
