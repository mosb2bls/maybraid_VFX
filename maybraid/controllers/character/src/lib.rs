//! Character controller: pad analog + edges → [`CharacterIntent`].

pub mod intent;
pub mod produce;

pub use intent::{CharacterIntent, InventoryCyclingCategory};
pub use maybraid_controllers_core::{
	ControllerPipeline, InputRouter, InputRouting, RoutingEvent, SourceConnection,
};

use bevy::prelude::*;
use maybraid_controllers_core::ControllersCorePlugin;
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

/// Bind-stage alias so existing `.after(CharacterControlSystems)` hosts keep working.
/// Removed in phase 2 once those hosts join [`ControllerPipeline::Bind`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControlSystems;

pub struct CharacterControllerPlugin;

impl Plugin for CharacterControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		app.add_message::<CharacterIntent>()
			.init_resource::<OrdinaryLocomotionEnabled>()
			.configure_sets(Update, CharacterControlSystems.in_set(ControllerPipeline::Bind))
			.add_systems(Update, produce_character_intents.in_set(CharacterControlSystems));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::{App, ResMut, Resource, Update};

	#[derive(Resource, Default)]
	struct StageOrder(Vec<&'static str>);

	fn note(label: &'static str) -> impl FnMut(ResMut<StageOrder>) {
		move |mut order: ResMut<StageOrder>| order.0.push(label)
	}

	#[test]
	fn character_bind_runs_inside_the_pipeline() {
		let mut app = App::new();
		ControllerPipeline::configure(&mut app);
		app.init_resource::<StageOrder>()
			.configure_sets(Update, CharacterControlSystems.in_set(ControllerPipeline::Bind))
			.add_systems(
				Update,
				(
					note("bind").in_set(CharacterControlSystems),
					note("interpret").in_set(ControllerPipeline::Interpret),
				),
			);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["bind", "interpret"]);
	}
}
