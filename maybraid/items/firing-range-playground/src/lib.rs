//! Firing range: pad + Les Halles stack → player + firearm-user plugins.

use std::time::Duration;

mod buildings_lod;
pub mod commands;
mod damage;
mod diagnostics;
mod engagement;
mod hit_debug;
mod hud;
mod les_halles;
mod loadout;
mod range;
mod session;
mod ui;
mod vantage;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use session::{FreeForAllBody, FreeForAllHost};

use bevy::ecs::schedule::common_conditions::resource_exists;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use building_components::{apply_parent_confines, FurnitureWireframePlugin, LabelWireframePlugin};
use building_physics::BuildingWalkColliderPlugin;
use buildings_lod::FiringRangeBuildingsLodPlugin;
use character_items::ItemRng;
use character_ragdoll::{CharacterRagdollPlugin, CharacterRagdollSettings};
use characters::CharacterHostsPlugin;
use combat_hud::CombatHudPlugin;
use diagnostics::FiringRangeDiagnosticsPlugin;
use evasion_intelligence::{EvasionPlugin, EvasionSystems};
use firearm_intelligence::{FirearmIntelligencePlugin, FirearmIntelligenceSystems};
use firearm_user::{spawn_reticle, FirearmUserPlugin};
use firearms::{
	add_firearm_components_host, FirearmHostsPlugin, FirearmWeaponSystems, FirearmWeaponsPlugin,
};
use fleeing_intelligence::{FleeingPlugin, FleeingSystems};
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use hiding_intelligence::{HidingPlugin, HidingSystems};
use les_halles::LesHallesSpawn;
use lod::LodRefreshSystems;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_input::{PadGameplayEnabled, VirtualPadSystems};
use movement_intelligence::{
	CandidateBudget, MovementIntelligenceLimits, MovementIntelligencePlugin,
};
use movement_intelligence_richmond::RichmondAvianMovementSurface;
use movement_realization::MovementRealizationPlugin;
use npc_intelligence::NpcIntelligencePlugin;
use player::{
	register_motor_traction_physics, spawn_npc_with_hidden_capsule, spawn_npc_with_hull,
	spawn_player_with_hidden_capsule, Npc, Player, PlayerLook, PlayerPlugin,
};
use player_camera::{spawn_follow_camera, PlayerCameraPlugin};
use session::{AppliedSession, Civilian, LoadoutRng, RangeMode, RangeSession};
use spotting_intelligence::SpottingSystems;
use threat_intelligence::{ThreatIntelligencePlugin, ThreatSystems};
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;
use threat_management_intelligence::ThreatManagementPlugin;

pub struct FiringRangePlugin;

impl Plugin for FiringRangePlugin {
	fn build(&self, app: &mut App) {
		register_motor_traction_physics(app);
		app.insert_resource(MovementIntelligenceLimits {
			max_budget: CandidateBudget { max_candidates: 12, max_steps: 3, horizon: 32.0 },
			..default()
		})
		.insert_resource(CharacterRagdollSettings {
			corpse_lifetime_secs: 5.0,
			max_simulation_secs: 1.0,
			..default()
		})
		.add_plugins(FirearmHostsPlugin);
		add_firearm_components_host::<firearm_user::GeneratedFirearm>(app);
		app.add_plugins(FirearmWeaponsPlugin)
			.add_plugins(CharacterHostsPlugin)
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(PlayerPlugin)
			.add_plugins(PlayerCameraPlugin)
			.add_plugins(FirearmUserPlugin)
			.add_plugins(CombatHudPlugin {
				hit_markers: true,
				directional_damage: true,
				player_vitals: true,
				lubikan_charge: false,
				lubikan_point_feedback: false,
				score: false,
			})
			.add_plugins(CharacterRagdollPlugin)
			.add_plugins(FiringRangeDiagnosticsPlugin)
			.add_plugins(FiringRangeBuildingsLodPlugin)
			.add_plugins((FurnitureWireframePlugin, LabelWireframePlugin))
			.add_plugins(BuildingWalkColliderPlugin)
			.add_plugins(
				MovementIntelligencePlugin::<RichmondAvianMovementSurface<'_, '_>>::default(),
			)
			.add_plugins(FirearmIntelligencePlugin)
			.add_plugins(ThreatIntelligencePlugin)
			.add_plugins(ThreatIntelligenceDamagePlugin)
			.add_plugins(ThreatManagementPlugin)
			.add_plugins(NpcIntelligencePlugin)
			.add_plugins(EvasionPlugin)
			.add_plugins(FleeingPlugin)
			.add_plugins(HidingPlugin)
			.add_plugins(MovementRealizationPlugin)
			.configure_sets(
				Update,
				SpottingSystems::Observe.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Spotting.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				FirearmIntelligenceSystems::Movement.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(EvasionSystems::Ingest, EvasionSystems::Rank)
					.chain()
					.after(SpottingSystems::Observe)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				Update,
				(FleeingSystems::Write, HidingSystems::Write)
					.chain()
					.after(EvasionSystems::Rank)
					.run_if(on_timer(Duration::from_millis(125))),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::ValidateAim.run_if(on_timer(Duration::from_millis(33))),
			)
			.configure_sets(
				PostUpdate,
				FirearmIntelligenceSystems::Fire.run_if(on_timer(Duration::from_millis(33))),
			)
			.init_resource::<LesHallesSpawn>()
			.init_resource::<damage::CombatRespawn>()
			.init_resource::<engagement::NpcEngagement>()
			.init_resource::<RangeSession>()
			.init_resource::<AppliedSession>()
			.insert_resource(LoadoutRng(ItemRng::from_entropy()))
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()))
			.add_systems(
				Startup,
				(
					spawn_follow_camera_system,
					setup_lighting,
					range::setup_range,
					spawn_reticle_system,
					hud::spawn_firing_range_hud,
				)
					.chain(),
			)
			.add_systems(
				PostStartup,
				(
					les_halles::setup_les_halles,
					session::apply_session,
					spawn_player_system,
					spawn_npc_system,
				)
					.chain(),
			)
			.add_systems(PreUpdate, gate_pad.before(VirtualPadSystems::Produce))
			.add_systems(Update, vantage::sync_range_threat_actors.in_set(ThreatSystems::Prepare))
			.add_systems(
				Update,
				vantage::seed_range_threat_observations
					.in_set(ThreatSystems::Ingest)
					.before(threat_intelligence::ingest_threat_observations),
			)
			.add_systems(
				Update,
				(
					session::apply_session,
					session::spawn_player_character,
					session::spawn_npc_character,
					session::spawn_held_system,
					(damage::queue_flee_out_respawns, respawn_combatants).chain(),
					vantage::sync_combat_spot_subjects
						.after(ThreatSystems::Discover)
						.before(SpottingSystems::Observe),
					les_halles::draw_circulation_gizmos,
					hit_debug::draw_dummy_hit_volumes,
					apply_parent_confines.after(LodRefreshSystems::Cull),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
					hud::sync_gun_stats,
				),
			)
			.add_systems(
				PostUpdate,
				damage::queue_downed_respawns.after(::damage::DamageSystems::Down),
			)
			.add_systems(
				PostUpdate,
				engagement::record_player_shot.after(::damage::DamageSystems::Collect),
			)
			.add_systems(
				PostUpdate,
				vantage::note_civilian_received_fire.after(FirearmWeaponSystems::Fire),
			);
	}
}

/// Inserted by the host app while the free-for-all should be running.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FreeForAllLive;

/// The Les Halles roster has been spawned for the current [`FreeForAllLive`].
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct FreeForAllMounted;

/// Directional lights spawned with the roster and removed when it leaves.
#[derive(Component)]
struct FreeForAllLight;

/// `mount_arena` inserts [`LesHallesSpawn`] through commands, so the roster waits
/// one frame until that pose is visible.
#[derive(Resource, Clone, Copy, Debug, Default)]
struct FreeForAllSpawning;

/// Free-for-all inside an app that already has the world intelligence stack.
///
/// Does not spawn at startup and does not register slash commands. [`FreeForAllLive`]
/// mounts the playground seed arena and the generated roster.
pub struct FreeForAllPlugin;

impl Plugin for FreeForAllPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<player::PlayerPlugin>() {
			app.add_plugins(player::PlayerPlugin);
		}
		app.init_resource::<LesHallesSpawn>()
			.init_resource::<damage::CombatRespawn>()
			.init_resource::<engagement::NpcEngagement>()
			.init_resource::<RangeSession>()
			.init_resource::<AppliedSession>()
			.insert_resource(LoadoutRng(ItemRng::from_entropy()))
			.init_resource::<FreeForAllHost>()
			.add_systems(Update, sync_free_for_all_mount)
			.add_systems(
				Update,
				(
					session::apply_session,
					session::spawn_player_character,
					session::spawn_npc_character,
					session::spawn_held_system,
					(damage::queue_flee_out_respawns, respawn_combatants).chain(),
					vantage::sync_combat_spot_subjects
						.after(ThreatSystems::Discover)
						.before(SpottingSystems::Observe),
					vantage::sync_range_threat_actors.in_set(ThreatSystems::Prepare),
					vantage::seed_range_threat_observations
						.in_set(ThreatSystems::Ingest)
						.before(threat_intelligence::ingest_threat_observations),
					apply_parent_confines.after(LodRefreshSystems::Cull),
				)
					.run_if(resource_exists::<FreeForAllLive>)
					.after(sync_free_for_all_mount),
			)
			.add_systems(
				PostUpdate,
				(
					damage::queue_downed_respawns.after(::damage::DamageSystems::Down),
					engagement::record_player_shot.after(::damage::DamageSystems::Collect),
					vantage::note_civilian_received_fire.after(FirearmWeaponSystems::Fire),
				)
					.run_if(resource_exists::<FreeForAllLive>),
			);
	}
}

fn sync_free_for_all_mount(
	live: Option<Res<FreeForAllLive>>,
	mounted: Option<Res<FreeForAllMounted>>,
	mut session: ResMut<RangeSession>,
	mut applied: ResMut<AppliedSession>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	spawning: Option<Res<FreeForAllSpawning>>,
	arena: Query<Entity, With<les_halles_arena::TrainingArena>>,
	bodies: Query<(Entity, Option<&firearm_user::FirearmUser>), With<FreeForAllBody>>,
	lights: Query<Entity, With<FreeForAllLight>>,
) {
	let want = live.is_some();
	let have = mounted.is_some();
	if !want {
		if have || spawning.is_some() {
			clear_free_for_all(&mut commands, &mut session, &mut applied, &bodies, &arena, &lights);
		}
		return;
	}
	if have {
		return;
	}
	if spawning.is_none() {
		les_halles_arena::mount_arena(
			&mut commands,
			&mut meshes,
			&mut materials,
			les_halles_arena::PLAYGROUND_SEED,
		);
		commands.insert_resource(FreeForAllSpawning);
		return;
	}
	session.enter_free_for_all(session::DEFAULT_FFA_NPCS, None);
	spawn_free_for_all_lights(&mut commands);
	commands.insert_resource(FreeForAllMounted);
	commands.remove_resource::<FreeForAllSpawning>();
}

fn clear_free_for_all(
	commands: &mut Commands,
	session: &mut RangeSession,
	applied: &mut AppliedSession,
	bodies: &Query<(Entity, Option<&firearm_user::FirearmUser>), With<FreeForAllBody>>,
	arena: &Query<Entity, With<les_halles_arena::TrainingArena>>,
	lights: &Query<Entity, With<FreeForAllLight>>,
) {
	for (entity, user) in bodies {
		if let Some(user) = user {
			commands.entity(user.held).try_despawn();
		}
		commands.entity(entity).despawn();
	}
	for entity in arena {
		commands.entity(entity).despawn();
	}
	for entity in lights {
		commands.entity(entity).despawn();
	}
	*session = RangeSession::default();
	applied.epoch = 0;
	commands.remove_resource::<FreeForAllMounted>();
	commands.remove_resource::<FreeForAllSpawning>();
}

fn spawn_free_for_all_lights(commands: &mut Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		FreeForAllLight,
		DirectionalLight { illuminance: 2500.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		FreeForAllLight,
		DirectionalLight { illuminance: 200.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 3.0, 0.0)),
	));
}

fn spawn_follow_camera_system(mut commands: Commands) {
	spawn_follow_camera(&mut commands);
}

fn spawn_player_system(
	session: Res<RangeSession>,
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if session.epoch != 0 {
		return;
	}
	spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
}

fn spawn_npc_system(
	session: Res<RangeSession>,
	mut commands: Commands,
	spawn: Res<LesHallesSpawn>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if session.epoch != 0 {
		return;
	}
	spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
}

pub(crate) fn spawn_player_at(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let player = spawn_player_with_hidden_capsule(commands, meshes, materials);
	commands.entity(player).insert((
		Transform::from_translation(spawn.player),
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		damage::Health::default(),
		damage::headshot_band(),
		FreeForAllBody,
	));
}

pub(crate) fn spawn_npc_at(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let npc = spawn_npc_with_hidden_capsule(
		commands,
		spawn.npc,
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		meshes,
		materials,
	);
	session::install_npc_combat(commands, npc, spawn.npc, None, None);
	commands.entity(npc).insert(FreeForAllBody);
}

fn dummy_translation(spawn: &LesHallesSpawn, hull: player::LocomotionCapsule) -> Vec3 {
	let mut at = spawn.npc;
	at.y += hull.spawn_height() - player::LocomotionCapsule::HUMANOID.spawn_height();
	at
}

pub(crate) fn spawn_dummy_at(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	species: session::DummySpecies,
	_meshes: &mut Assets<Mesh>,
	_materials: &mut Assets<StandardMaterial>,
) {
	let hull = species.hull();
	let at = dummy_translation(spawn, hull);
	let dummy =
		spawn_npc_with_hull(commands, at, PlayerLook { yaw: spawn.look_yaw, ..default() }, hull);
	commands.entity(dummy).insert((
		damage::Health::default(),
		damage::headshot_band_for(hull),
		session::TestDummy,
		FreeForAllBody,
		species,
	));
}

#[allow(clippy::too_many_arguments)]
fn respawn_combatants(
	time: Res<Time>,
	spawn: Res<LesHallesSpawn>,
	session: Res<RangeSession>,
	players: Query<(), (With<Player>, With<FreeForAllBody>, Without<::damage::Downed>)>,
	combatants: Query<
		(),
		(With<Npc>, With<FreeForAllBody>, Without<Civilian>, Without<::damage::Downed>),
	>,
	civilians: Query<(), (With<Civilian>, With<FreeForAllBody>, Without<::damage::Downed>)>,
	mut respawn: ResMut<damage::CombatRespawn>,
	mut rng: ResMut<LoadoutRng>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let now = time.elapsed_secs();
	if players.is_empty() && respawn.player_at.is_some_and(|at| now >= at) {
		respawn.player_at = None;
		match session.mode {
			RangeMode::FreeForAll | RangeMode::AssaultFreeForAll => {
				session::spawn_generated_player(
					&mut commands,
					&spawn,
					&mut rng.0,
					&mut meshes,
					&mut materials,
				);
			}
			RangeMode::Duel | RangeMode::TestDummy => {
				spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
			}
		}
	}
	let mut due = 0usize;
	respawn.npc_at.retain(|at| {
		if now >= *at {
			due += 1;
			false
		} else {
			true
		}
	});
	let live = combatants.iter().count();
	let want = session.npc_count as usize;
	let n = due.min(want.saturating_sub(live));
	let total = session.npc_count + session.civilian_count;
	for index in 0..n {
		let slot = (live + index) as u16;
		match session.mode {
			RangeMode::FreeForAll | RangeMode::AssaultFreeForAll => {
				session::spawn_generated_npc(
					&mut commands,
					&spawn,
					slot,
					if session.is_assault_free_for_all() { total } else { session.npc_count },
					&mut rng.0,
					&mut meshes,
					&mut materials,
				);
			}
			RangeMode::Duel => {
				spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
			}
			RangeMode::TestDummy => {
				spawn_dummy_at(
					&mut commands,
					&spawn,
					session.dummy_species,
					&mut meshes,
					&mut materials,
				);
			}
		}
	}

	let mut civilian_due = 0usize;
	respawn.civilian_at.retain(|at| {
		if now >= *at {
			civilian_due += 1;
			false
		} else {
			true
		}
	});
	let live_civilians = civilians.iter().count();
	let want_civilians = session.civilian_count as usize;
	let n_civilians = civilian_due.min(want_civilians.saturating_sub(live_civilians));
	for index in 0..n_civilians {
		let slot = session.npc_count + (live_civilians + index) as u16;
		session::spawn_generated_civilian(
			&mut commands,
			&spawn,
			slot,
			total.max(1),
			&mut meshes,
			&mut materials,
		);
	}
}

fn spawn_reticle_system(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

fn gate_pad(focus: Res<TextEntryFocus>, mut enabled: ResMut<PadGameplayEnabled>) {
	enabled.0 = !focus.0;
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 2500.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 200.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 3.0, 0.0)),
	));
}
