//! Character controller: pad analog + edges → [`CharacterIntent`].

pub mod intent;
pub mod produce;
pub mod sample;

pub use intent::{CharacterIntent, InventoryCyclingCategory};
pub use maybraid_controllers_core::{
	ActorTarget, ControllerPipeline, InputRouter, InputSource, Routed,
};
pub use sample::{
	produce_actor_control_samples, ActorControlSample, FaceButtonSample, RoutedActorControl,
};

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureResolveSystems, ControllersCorePlugin};
use maybraid_input::VirtualPadPlugin;

use crate::produce::produce_character_intents;

/// Per-actor ordinary locomotion participation. Missing means enabled.
/// Tanz-user writes this once on spawn and again only when that actor's
/// Standard ↔ Tanzer mode actually changes.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrdinaryLocomotion(pub bool);

impl Default for OrdinaryLocomotion {
	fn default() -> Self {
		Self(true)
	}
}

/// Bind-stage alias so leftover `.after(CharacterControlSystems)` item / camera
/// / shell hosts keep working. Motor hosts join [`ControllerPipeline`] sets.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CharacterControlSystems;

/// Ordered Bind work inside [`CharacterControlSystems`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CharacterBindSystems {
	/// Stamp [`RoutedActorControl`] from the local pad.
	Route,
	/// Broadcast [`CharacterIntent`] (still untargeted).
	Produce,
}

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
			.init_resource::<RoutedActorControl>()
			.configure_sets(Update, CharacterControlSystems.in_set(ControllerPipeline::Bind))
			.configure_sets(
				Update,
				(
					CharacterBindSystems::Route
						.in_set(CharacterControlSystems)
						.after(CaptureResolveSystems),
					CharacterBindSystems::Produce
						.in_set(CharacterControlSystems)
						.after(CharacterBindSystems::Route),
				),
			)
			.add_systems(Update, produce_actor_control_samples.in_set(CharacterBindSystems::Route))
			.add_systems(Update, produce_character_intents.in_set(CharacterBindSystems::Produce));
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
