//! Assembled world model: Durham terrain, streamed forest, urbanization, sky dome.
//!
//! Character mode is the default. Forest grove fill is 1 km present / 3 km
//! selection generate. Canopy bump-outs occupy the 1–5 km present keep and
//! clone Durham fine-cell mesh handles. Vegetation LOD bullseye / lattice
//! cover the grove fill ring. Urbanization hopscotch streams at the same
//! 1 km / 3 km rings without re-registering Durham (base terrain generation owns
//! the fill; raw present is a separate plugin, off in the world).
//! Painted furniture is a generate pass over Richmond High slots, presented as
//! flattened 50 m cell hosts in a neighborhood around the camera.

mod camera;
pub mod commands;
mod control;
mod crate_loot;
mod first_wave;
mod intelligence;
mod lubikan_activate;
mod map_view;
mod pitch;
mod player_lifecycle;
mod player_position;
mod poi;
mod start;
mod stash;
mod ui;
mod vsync;
mod weapon;
mod world_furnished;
mod world_layers;

pub use world_furnished::WorldFurnished;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use control::{
	InventoryEditCameraFollow, WorldGameplayEnabled, WorldPlaySessionEnded, WorldSceneryVisible,
	WorldSurfaceReady, WorldSurfaceSet,
};
pub use durham::Durham;
#[cfg(any(test, feature = "test-support"))]
pub use first_wave::UnitXTile;
pub use first_wave::{FirstWave, FIRST_WAVE_JOB_THRESHOLD};
pub use game_commands::command::PendingStartupCommand;
pub use intelligence::WorldIntelligencePlugin;
pub use lod::hcsg::{HcsgClass, HcsgDemand, Outstanding};
pub use lod::LodJobCounter;
pub use map_view::{WorldMapView, WorldMapViewPlugin};
pub use maybraid_sky::{
	ShadowQuality, SkyClock, SkyCommand, SKY_BLUE, SKY_CLEAR, SKY_HORIZON, SKY_NADIR, SKY_ZENITH,
	SUN_COLOR, SUN_ILLUMINANCE,
};
pub use player_camera::{CameraPov, CameraPovLocked};
pub use player_lifecycle::{
	reset_first_spawn_offer, WorldPlayerLifecyclePlugin, WorldPlayerRespawnConfig,
};
pub use player_position::{
	resume_discovery_from_saved_waypoints, PlayerPositionPlugin, PlayerPositionWaypoints,
};
pub use poi::{WorldPoiDiscoveryBudget, WorldPoiPlugin, WorldPoiSystems};
pub use start::{
	parse_xz_metres, player_spawn_xz, resolve_start_at, start_at_from_env, take_start_at_from_args,
	START_AT_ENV,
};
pub use stash::{
	spawn_exploded_stashes, spawn_world_stash, StashDisplayedItem, StashPolicy, WorldStash,
	WorldStashPlugin, WorldStashSettings, DEFAULT_CLAIM_RADIUS, DEFAULT_LOOT_SECS,
};
pub use terrain_layer_model::{terrain_streaming, TerrainStreaming};
pub use ui::WorldMobHudEnabled;
pub use vsync::{default_window_present_mode, RequestVsyncToggle, VSYNC_TOGGLE_KEY};
pub use weapon::WorldPlayerLoadout;
pub use world_layers::WorldLayersPlugin;
pub use world_materials::{WorldMaterialLib, WorldMaterialRefPlugin};
pub use world_player::{PlayerPhysicsEnabled, PlayerSpawnXz};

use avian3d::prelude::{CoefficientCombine, Friction};
use bevy::prelude::*;
use character_ragdoll::{CharacterRagdollPlugin, CharacterRagdollTargets};
use characters::{CharacterMotionSystems, DrawTerrainPitchProbes};
use combat_hud::CombatHudPlugin;
use durham::TerrainFrictionConfig;
use firearms::EnergyMaterialPlugin;
use game_commands::command::{GameCommandPlugin, TextEntryFocus};
use game_commands::ui::GameCommandDrawerConfig;
use lod::{Bullseye, OpenLattice};
use lubikans::{LubikanSystems, LubikansPlugin};
use lubikans_damage::LubikanDamagePlugin;
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};
use maybraid_game_mode_discover::DiscoveryPlayerPlugin;
use maybraid_input::{VirtualPadConfig, VirtualPadPlugin};
use maybraid_skill_map::{SkillMapPlugin, SkillMapSystems};
use maybraid_sky::SkyDomePlugin;
use maybraid_vfx::VfxPlugin;
use maybraid_weather::WeatherPlugin;
use player::{
	register_motor_traction_physics, PlayerPlugin, PlayerPresentationPlugin, PlayerSystems,
};
use player_camera::{PlayerCameraPlugin, PlayerCameraSystems};
use world_player::{
	CharacterCameraFollowEnabled, CharacterLocomotion, CharacterSpecies, MeshStatsPlugin,
	PadMovementEnabled, PlayerControlSystems, PlaygroundDiag, PlaygroundMode,
	PlaygroundTimingPlugin, RequestSetCharacter, VegetationHostPlugin, VegetationPlayerMotor,
};

/// Steepest slope the controlled character can drive uphill.
const WORLD_MAX_SLOPE_ANGLE: f32 = 70.0_f32.to_radians();
/// Static grip for mobs and props. Motor-driven capsules zero contact friction
/// through [`player::MotorTractionHooks`].
const WORLD_TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 2.55,
	static_coefficient: 2.95,
	combine_rule: CoefficientCombine::Max,
};

/// ±1 km so produce covers the 1 km grove present ring.
const WORLD_BULLSEYE_OUTER_M: f32 = 2_000.0;
/// Cull annulus starts beyond the present ring.
const WORLD_LATTICE_EXCLUDE_M: f32 = 2_000.0;
const WORLD_LATTICE_OUTER_M: f32 = 8_000.0;
const WORLD_COMBAT_HUD: CombatHudPlugin = CombatHudPlugin {
	health_bars: false,
	hit_markers: true,
	directional_damage: true,
	player_vitals: true,
	lubikan_charge: true,
	lubikan_point_feedback: true,
	score: true,
};
const WORLD_TERRAIN_PITCH_GIZMOS: DrawTerrainPitchProbes = DrawTerrainPitchProbes(false);

/// Assembled world: Durham terrain, streamed forest, urbanization, sky dome, character.
///
/// Playground chrome (command drawer and FPS HUD) is on by default.
/// The game executable uses [`WorldPlugin::game`] (FPS log, no HUD or console).
/// `F8` / `/stats vsync` / `MAYBRAID_VSYNC=off` toggles vsync for Tracy flights.
pub struct WorldPlugin {
	/// `/` console, debug gizmos, and FPS HUD.
	pub debug_chrome: bool,
	/// Throttled `[timing]` FPS log ([`PlaygroundTimingPlugin`]).
	pub fps_diag: bool,
	/// Upper-left virtual-pad / command-intent dump.
	pub input_debug_enabled: bool,
}

impl Default for WorldPlugin {
	fn default() -> Self {
		Self { debug_chrome: true, fps_diag: true, input_debug_enabled: false }
	}
}

impl WorldPlugin {
	/// World systems without playground overlays. FPS log stays on.
	pub fn game() -> Self {
		Self { debug_chrome: false, fps_diag: true, input_debug_enabled: false }
	}
}

impl Plugin for WorldPlugin {
	fn build(&self, app: &mut App) {
		register_motor_traction_physics(app);
		app.init_resource::<lod::LodJobCounter>()
			.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlaygroundDiag {
				fps: self.fps_diag || self.debug_chrome,
				hud: self.debug_chrome,
			})
			.insert_resource(CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(player::CharacterLocomotion { max_slope_angle: WORLD_MAX_SLOPE_ANGLE })
			.insert_resource(TerrainFrictionConfig(WORLD_TERRAIN_FRICTION))
			.insert_resource(WORLD_TERRAIN_PITCH_GIZMOS)
			.add_plugins(WorldMaterialRefPlugin)
			.add_plugins(EnergyMaterialPlugin)
			.add_plugins(VfxPlugin)
			.add_plugins(VirtualPadPlugin::new(VirtualPadConfig {
				debug_overlay: self.input_debug_enabled,
				..default()
			}))
			.add_plugins(CharacterControllerPlugin)
			.add_plugins(PlayerPresentationPlugin)
			.add_plugins(PlayerCameraPlugin)
			.add_plugins(WORLD_COMBAT_HUD)
			.add_plugins(VegetationHostPlugin { register_camera: false })
			.add_plugins(MeshStatsPlugin)
			.insert_resource(CharacterRagdollTargets { players: true, npcs: true, unmarked: false })
			.add_plugins(CharacterRagdollPlugin)
			.add_plugins(WorldLayersPlugin)
			.add_plugins(DiscoveryPlayerPlugin)
			.add_plugins(WorldIntelligencePlugin)
			.add_plugins(WeatherPlugin)
			.add_plugins(SkillMapPlugin)
			.add_plugins(LubikansPlugin)
			.add_plugins(LubikanDamagePlugin)
			.insert_resource(maybraid_skill_map::SkillMapEnabled(false))
			.add_plugins(WorldPoiPlugin)
			.add_plugins(WorldMapViewPlugin)
			.add_plugins(WorldPlayerLifecyclePlugin)
			.add_plugins(WorldStashPlugin)
			.add_plugins(PlayerPositionPlugin)
			.insert_resource(PadMovementEnabled(false))
			.insert_resource(VegetationPlayerMotor(false))
			.insert_resource(CharacterCameraFollowEnabled(false))
			.add_message::<control::WorldPlaySessionEnded>()
			.init_resource::<WorldGameplayEnabled>()
			.init_resource::<InventoryEditCameraFollow>()
			.init_resource::<WorldSurfaceReady>()
			.init_resource::<crate::FirstWave>()
			.init_resource::<WorldSceneryVisible>()
			.insert_resource(WorldMobHudEnabled::from_debug_chrome(self.debug_chrome))
			.insert_resource(Bullseye { inner: 50.0, outer: WORLD_BULLSEYE_OUTER_M })
			.insert_resource(OpenLattice {
				exclude_extent: WORLD_LATTICE_EXCLUDE_M,
				outer_extent: WORLD_LATTICE_OUTER_M,
				tile_size: 500.0,
			})
			.add_plugins(SkyDomePlugin::default());
		if self.fps_diag || self.debug_chrome {
			app.add_plugins(PlaygroundTimingPlugin);
		}
		if self.debug_chrome {
			app.add_plugins(
				GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
					.with_drawer_config(GameCommandDrawerConfig {
						open_at_start: false,
						toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
						toggle_gamepad_buttons: Vec::new(),
					}),
			);
		} else {
			app.init_resource::<TextEntryFocus>();
		}
		app.add_systems(Startup, vsync::apply_startup_vsync)
			.add_systems(Update, (vsync::toggle_vsync, commands::apply_sky_commands))
			.add_systems(PostStartup, spawn_default_braidman)
			.add_systems(
				PreUpdate,
				(
					control::stamp_vegetation_motor_traction,
					control::stamp_world_player_motor,
					control::grant_world_player_lubikan_owner.before(LubikanSystems::Owner),
					control::sync_lubikan_session.before(LubikanSystems::Owner),
				),
			)
			.configure_sets(Update, control::WorldSurfaceSet)
			.add_systems(
				Update,
				(
					control::update_world_surface_ready.in_set(control::WorldSurfaceSet),
					control::sync_world_scenery,
					control::sync_combat_hud_visible,
					control::sync_skill_map_enabled.before(SkillMapSystems::Spawn),
					control::apply_intents_to_movement
						.after(CharacterControlSystems)
						.after(map_view::WorldMapSet::Toggle)
						.after(PlayerSystems::Intent)
						.before(PlayerSystems::Body)
						.before(PlayerControlSystems),
				),
			);
		camera::configure(app);
		weapon::configure(app);
		app.configure_sets(
			Update,
			PlayerCameraSystems::Body
				.after(CharacterMotionSystems::Anim)
				.before(CharacterMotionSystems::Elevation),
		);
		// After [`PlayerCameraPlugin`]: that plugin adds Camera Body → Locomotion
		// only when [`PlayerPlugin`] is already present, which cycles with
		// Locomotion → Anim → Camera Body.
		if !app.is_plugin_added::<PlayerPlugin>() {
			app.add_plugins(PlayerPlugin);
		}
		app.add_systems(
			Update,
			(
				pitch::sync_suspend_terrain_pitch.after(PlayerControlSystems),
				pitch::apply_avian_terrain_pitch
					.in_set(CharacterMotionSystems::Elevation)
					.after(pitch::sync_suspend_terrain_pitch),
			),
		);
		app.add_systems(
			Update,
			(
				ui::sync_mob_debug_hud_presence,
				ui::sync_mob_debug_pins.run_if(resource_equals(WorldMobHudEnabled(true))),
				ui::draw_mob_debug_gizmos.run_if(resource_equals(WorldMobHudEnabled(true))),
				ui::draw_npc_behavior_gizmos.run_if(resource_equals(WorldMobHudEnabled(true))),
			),
		);
		if self.debug_chrome {
			app.add_systems(Startup, ui::spawn_mob_debug_hud).add_systems(
				Update,
				(
					control::echo_character_intents
						.after(CharacterControlSystems)
						.before(game_commands::ui::update_debug_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
		}
		lubikan_activate::configure(app);
	}
}

fn spawn_default_braidman(mut commands: Commands) {
	commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_slope_is_below_wall_grade() {
		let degrees = WORLD_MAX_SLOPE_ANGLE.to_degrees();
		assert!(degrees < 80.0);
		assert!(degrees > 55.0);
	}

	#[test]
	fn world_terrain_static_friction_holds_max_walkable_slope() {
		assert!(WORLD_TERRAIN_FRICTION.static_coefficient > WORLD_MAX_SLOPE_ANGLE.tan());
	}

	#[test]
	fn world_input_debug_overlay_is_opt_in() {
		assert!(!WorldPlugin::default().input_debug_enabled);
		assert!(!WorldPlugin::game().input_debug_enabled);
	}

	#[test]
	fn game_world_keeps_fps_log_without_hud() {
		let game = WorldPlugin::game();
		assert!(game.fps_diag);
		assert!(!game.debug_chrome);
	}

	#[test]
	fn world_only_enables_combat_feedback_without_health_bars() {
		assert!(!WORLD_COMBAT_HUD.health_bars);
		assert!(WORLD_COMBAT_HUD.hit_markers);
		assert!(WORLD_COMBAT_HUD.directional_damage);
		assert!(WORLD_COMBAT_HUD.player_vitals);
		assert!(WORLD_COMBAT_HUD.lubikan_charge);
		assert!(WORLD_COMBAT_HUD.lubikan_point_feedback);
	}

	#[test]
	fn world_disables_terrain_pitch_gizmos() {
		assert!(!WORLD_TERRAIN_PITCH_GIZMOS.0);
	}

	#[test]
	fn game_world_starts_with_mob_hud_off() {
		assert!(!WorldMobHudEnabled::from_debug_chrome(WorldPlugin::game().debug_chrome).0);
		assert!(WorldMobHudEnabled::from_debug_chrome(WorldPlugin::default().debug_chrome).0);
	}

	#[test]
	fn world_starts_with_skill_maps_gated() {
		assert!(!maybraid_skill_map::SkillMapEnabled(false).0);
	}

	#[test]
	fn late_player_locomotion_does_not_cycle_camera_body_after_anim() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).configure_sets(
			Update,
			(
				PlayerSystems::Locomotion
					.after(PlayerSystems::Body)
					.before(CharacterMotionSystems::Anim),
				CharacterMotionSystems::Elevation.after(CharacterMotionSystems::Anim),
				PlayerCameraSystems::Body
					.after(CharacterMotionSystems::Anim)
					.before(CharacterMotionSystems::Elevation),
			),
		);
		app.update();
	}
}
