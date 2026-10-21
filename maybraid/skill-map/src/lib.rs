//! Discover skill maps: a 2D paradimension a [`SkillMapUser`] steers with stick
//! flicks while **RB** is held. Either stick; a hold is not a flick. Walks claim
//! authored tiles that dispatch world effects.
//!
//! Stamp [`SkillMapUser`] on the live character (same install as
//! [`firearm_user::FirearmUser`]). Pause / text-entry should clear
//! [`SkillMapEnabled`].

mod burst;
mod controller;
mod cursor;
mod effects;
mod fireball_embers;
mod fireball_material;
mod fireball_trail;
mod icon;
mod map;
mod preview;
mod tile_material;
mod tiles;
mod user;
mod viewport;

use bevy::prelude::*;
use fireball_material::FireballMaterialPlugin;

use maybraid_character_controller::CharacterControlSystems;
use maybraid_controllers_core::RoutingEvent;
use projectiles::ProjectilesPlugin;
use threat_management_intelligence::ThreatManagementSystems;

pub use burst::{
	launch_vy, COSIMO_BURST_DAMAGE, COSIMO_BURST_RADIUS, COSIMO_LAUNCH_HEIGHT, ROCKADDER_DAMAGE,
	ROCKADDER_RADIUS,
};
pub use controller::{
	detect_skill_map_flicks, skill_map_steer_held, SkillMapController, SkillMapFlick,
	SkillMapSteerPolicy,
};
pub use cursor::apply_flicks;
pub use effects::{
	forget_chance, FIREBALL_COLOR, FIREBALL_GRAVITY, FIREBALL_RADIUS, FIREBALL_SPEED,
};
pub use map::{authored_map, authored_map_from_spec, authored_maps, SkillKind, SkillMapId};
pub use preview::{
	spawn_skill_map_catalog_preview, spawn_skill_map_spin_reveal_hud,
	spawn_skill_map_spin_reveal_image, SkillMapCatalogPreview, SkillMapMenuPreview,
	SkillMapSpinRevealHud, SkillMapSpinRevealHudView, CATALOG_PREVIEW_PX,
};
pub use tile_material::{SkillMapTileAssets, SkillMapTileMaterialPlugin};
pub use tiles::{classify_noise, TileKind};
pub use user::{
	spawn_skill_maps, spawn_skill_maps_with, MappedBy, SkillMapEquip, SkillMapHeld, SkillMapMember,
	SkillMapSession, SkillMapSteerLock, SkillMapUser, SkillMapUserSettings,
};
pub use viewport::{
	spawn_skill_map_view, Debraid, SkillMapFirearmHud, SkillMapViewport, SkillMapViewportCamera,
	SpawnedSkillMapView, SKILL_MAP_RB_ICON, SKILL_MAP_RT_ICON, SKILL_MAP_Y_ICON,
};

/// When `false`, the map is hidden and claims / steering are ignored.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillMapEnabled(pub bool);

impl Default for SkillMapEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// One claimed power tile or a water fail, attributed to the mapping user.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillMapEvent {
	Claim { user: Entity, kind: SkillKind },
	Fail { user: Entity, map: SkillMapId },
}

/// Dispatch systems drain pending claims while the map is hidden.
pub(crate) fn skill_map_dispatch_ready(
	enabled: &SkillMapEnabled,
	events: &mut MessageReader<SkillMapEvent>,
) -> bool {
	if enabled.0 {
		return true;
	}
	for _ in events.read() {}
	false
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillMapSystems {
	Spawn,
	Steer,
	Collide,
	Dispatch,
}

pub struct SkillMapPlugin;

impl Plugin for SkillMapPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ProjectilesPlugin>() {
			app.add_plugins(ProjectilesPlugin);
		}
		if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
			app.add_plugins(bevy_hanabi::HanabiPlugin);
		}
		app.add_plugins(FireballMaterialPlugin)
			.add_plugins(SkillMapTileMaterialPlugin)
			.add_systems(Startup, fireball_embers::setup_fireball_effects)
			.add_systems(Startup, burst::setup_burst_assets)
			.init_resource::<SkillMapEnabled>()
			.init_resource::<controller::SkillMapSteerPolicy>()
			.init_resource::<controller::SkillMapController>()
			.init_resource::<icon::SkillMapPaintSlots>()
			.init_resource::<icon::SkillMapIconSubmitted>()
			.add_message::<SkillMapEvent>()
			.add_message::<RoutingEvent>()
			.add_message::<controller::SkillMapFlick>()
			.configure_sets(
				Update,
				(
					SkillMapSystems::Spawn.after(CharacterControlSystems),
					SkillMapSystems::Steer.after(SkillMapSystems::Spawn),
					SkillMapSystems::Collide.after(SkillMapSystems::Steer),
					SkillMapSystems::Dispatch
						.after(SkillMapSystems::Collide)
						.before(ThreatManagementSystems::Select),
				),
			)
			.add_systems(Update, icon::realize_skill_map_capture_hosts)
			.add_systems(Update, viewport::present_skill_maps.in_set(SkillMapSystems::Spawn))
			.add_systems(
				Update,
				(
					controller::detect_skill_map_flicks,
					cursor::sync_skill_map_held,
					cursor::tick_steer_lock,
					cursor::apply_flicks,
					cursor::tick_flick_beads,
					viewport::sync_viewport_chrome,
					viewport::sync_firearm_chrome,
					viewport::sync_grenade_chrome,
					viewport::track_cursors,
					viewport::tick_debraid,
				)
					.chain()
					.in_set(SkillMapSystems::Steer),
			)
			.add_systems(Update, tiles::collide_tiles.in_set(SkillMapSystems::Collide))
			.add_systems(
				Update,
				(
					tiles::restore_spent_tiles,
					effects::dispatch_fireballs,
					effects::dispatch_dumbwaves,
					burst::dispatch_rockadders,
					burst::dispatch_cosmos,
					effects::tick_pulses,
					burst::tick_bursts,
					burst::tick_shards,
					burst::contact_bursts,
					user::despawn_orphaned_skill_maps,
				)
					.chain()
					.in_set(SkillMapSystems::Dispatch),
			)
			.add_systems(
				PostUpdate,
				(
					fireball_trail::drop_fireball_beads,
					fireball_trail::tick_fireball_beads,
					fireball_trail::contact_fireball_beads,
				)
					.chain()
					.after(TransformSystems::Propagate),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn gameplay_off_is_an_explicit_gate() {
		assert!(SkillMapEnabled::default().0);
		assert!(!SkillMapEnabled(false).0);
	}

	#[test]
	fn authored_maps_cover_the_four_base_kinds() {
		let maps = authored_maps();
		assert_eq!(maps.len(), 4);
		assert_eq!(maps[0].kind, SkillKind::Fireball);
		assert_eq!(maps[1].kind, SkillKind::Dumbwave);
		assert_eq!(maps[2].kind, SkillKind::Rockadder);
		assert_eq!(maps[3].kind, SkillKind::Cosimo);
		assert_ne!(SkillKind::Fireball.viewport_clear(), SkillKind::Dumbwave.viewport_clear());
		assert_ne!(SkillKind::Rockadder.viewport_clear(), SkillKind::Cosimo.viewport_clear());
		let seeded = authored_map(SkillKind::Fireball, 99);
		assert_eq!(seeded.seed, 99);
		assert_eq!(seeded.id, SkillMapId(0));
	}
}
