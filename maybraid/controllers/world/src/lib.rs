//! World controller: session capabilities, pause / live-play, and suspension.

pub mod capabilities;
pub mod enabled;

pub use capabilities::{Participation, SessionBit, SessionCapabilities};
pub use enabled::WorldGameplayEnabled;
pub use maybraid_controllers_core::{
	publish_routing_events, CaptureKind, CaptureToken, InputRouter, InputRouting, RoutingEvent,
	ScopeOwner, Suspension, SuspensionToken,
};

use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;

/// Apply pending pause / capture projections after Bind and before prohibited
/// realization (WorldFilter, items, map).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionCapabilitySystems;

pub struct WorldControllerPlugin;

impl Plugin for WorldControllerPlugin {
	fn build(&self, app: &mut App) {
		ControllerPipeline::configure(app);
		app.add_message::<RoutingEvent>()
			.init_resource::<InputRouter>()
			.add_systems(Last, publish_routing_events)
			.init_resource::<WorldGameplayEnabled>()
			.init_resource::<SessionCapabilities>()
			.configure_sets(
				Update,
				SessionCapabilitySystems
					.after(ControllerPipeline::Bind)
					.before(ControllerPipeline::Interpret),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::{App, ResMut, Resource, Update};
	use maybraid_controllers_core::{InputRouter, InputRouting, ScopeOwner};

	#[derive(Resource, Default)]
	struct StageOrder(Vec<&'static str>);

	fn note(label: &'static str) -> impl FnMut(ResMut<StageOrder>) {
		move |mut order: ResMut<StageOrder>| order.0.push(label)
	}

	#[test]
	fn session_capabilities_run_after_bind_before_interpret() {
		let mut app = App::new();
		app.add_plugins(WorldControllerPlugin)
			.init_resource::<StageOrder>()
			.add_systems(
				Update,
				(
					note("bind").in_set(ControllerPipeline::Bind),
					note("session").in_set(SessionCapabilitySystems),
					note("interpret").in_set(ControllerPipeline::Interpret),
				),
			);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["bind", "session", "interpret"]);
	}

	#[test]
	fn plugin_installs_router_and_session() {
		let mut app = App::new();
		app.add_plugins(WorldControllerPlugin);
		assert!(app.world().get_resource::<InputRouter>().is_some());
		assert!(app.world().get_resource::<SessionCapabilities>().is_some());
	}

	#[test]
	fn plugin_schedule_honors_a_pause_token_the_same_frame() {
		fn apply_pause(
			mut capabilities: ResMut<SessionCapabilities>,
			mut routing: InputRouting,
			mut gameplay: ResMut<WorldGameplayEnabled>,
		) {
			if !capabilities.gameplay().is_installed() {
				capabilities.install_play(true);
			}
			routing.set_paused(true);
			*gameplay = WorldGameplayEnabled::from_session(*capabilities, &routing);
		}

		let mut app = App::new();
		app.add_plugins(WorldControllerPlugin)
			.add_systems(Update, apply_pause.in_set(SessionCapabilitySystems));
		app.update();
		assert!(!app.world().resource::<WorldGameplayEnabled>().0);
		assert!(app.world().resource::<SessionCapabilities>().physics_installed());
		assert!(app.world().resource::<SessionCapabilities>().gameplay().is_installed());
		assert!(app.world().resource::<InputRouter>().has_suspension(
			maybraid_controllers_core::InputSource::local_pad(),
			ScopeOwner::pause()
		));
	}
}
