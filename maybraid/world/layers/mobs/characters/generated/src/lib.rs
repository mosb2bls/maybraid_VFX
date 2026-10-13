//! Generated mob characters: build, species, inventory, brains, and real scenes.

mod brain;
mod build;
mod inventory;
mod number;
mod scene;
mod species;

use bevy::prelude::*;
use character_inventory_user::InventoryUserPlugin;
use characters::CharacterHostsPlugin;
use firearms::{add_firearm_components_host, FirearmHostsPlugin};
use npc_intelligence::NpcIntelligencePlugin;
use player::PlayerPlugin;

pub use brain::{CharacterBrains, CHARACTER_POI, LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
pub use build::CharacterBuild;
pub use firearm_user::GeneratedFirearm;
pub use inventory::CharacterInventory;
pub use number::FromMobNumber;
pub use scene::{CharacterSceneRecipe, CharacterSceneSystems, CharacterStats, MobCharacter};
pub use species::CharacterSpecies;

/// Registers nested character/firearm hosts and materializes scene plants.
///
/// Applications still own movement surfaces and intelligence execution plugins;
/// this plugin installs the per-entity users consumed by those systems.
pub struct MobCharacterScenesPlugin;

impl Plugin for MobCharacterScenesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<CharacterHostsPlugin>() {
			app.add_plugins(CharacterHostsPlugin);
		}
		if !app.is_plugin_added::<FirearmHostsPlugin>() {
			app.add_plugins(FirearmHostsPlugin);
		}
		if !app.is_plugin_added::<InventoryUserPlugin>() {
			app.add_plugins(InventoryUserPlugin);
		}
		if !app.is_plugin_added::<PlayerPlugin>() {
			app.add_plugins(PlayerPlugin);
		}
		if !app.is_plugin_added::<NpcIntelligencePlugin>() {
			app.add_plugins(NpcIntelligencePlugin);
		}
		add_firearm_components_host::<GeneratedFirearm>(app);
		scene::configure_character_scene_systems(app);
	}
}
