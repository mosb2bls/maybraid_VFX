//! Character controller: pad analog + edges → [`CharacterIntent`].

pub mod intent;
pub mod produce;

pub use intent::{CharacterIntent, InventoryCyclingCategory};

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;

use crate::produce::produce_character_intents;

/// When `false`, pad move / jump / stance / sprint are not turned into intents.
/// Look and item controls stay live. Tanz-user clears this in Tanzer mode.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryLocomotionEnabled(pub bool);

impl Default for OrdinaryLocomotionEnabled {
	fn default() -> Self {
		Self(true)
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControlSystems;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		app.add_message::<CharacterIntent>()
			.init_resource::<OrdinaryLocomotionEnabled>()
			.add_systems(Update, produce_character_intents.in_set(CharacterControlSystems));
	}
}
