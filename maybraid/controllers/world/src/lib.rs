//! World controller: pause / live-play gate for character input.

pub mod enabled;

pub use enabled::WorldGameplayEnabled;

use bevy::prelude::*;

pub struct WorldControllerPlugin;

impl Plugin for WorldControllerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldGameplayEnabled>();
	}
}
