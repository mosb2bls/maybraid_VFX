//! Character controller: pad analog + edges → [`CharacterIntent`].

pub mod intent;
pub mod produce;

pub use intent::{CharacterIntent, InventoryCyclingCategory};

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;

use crate::produce::produce_character_intents;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControlSystems;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		app.add_message::<CharacterIntent>()
			.add_systems(Update, produce_character_intents.in_set(CharacterControlSystems));
	}
}
