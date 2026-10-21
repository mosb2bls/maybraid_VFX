//! Session capabilities, pause / live-play, and play-session lifecycle.

pub mod capabilities;
pub mod lifecycle;
pub mod suspension;

pub use capabilities::{Participation, SessionBit, SessionCapabilities};
pub use lifecycle::{ActorRetired, SessionLifecycleSystems, WorldPlaySessionEnded};
pub use maybraid_controllers_core::{
	publish_routing_events, CaptureKind, CaptureToken, InputRouter, InputRouting, RoutingEvent,
	ScopeOwner, Suspension, SuspensionToken,
};
pub use suspension::{apply_in_game_menu_pause, WorldGameplayEnabled};

use bevy::prelude::*;
use maybraid_controllers_core::ControllerPipeline;

/// Apply pending pause / capture projections after Bind and before prohibited
/// realization (WorldFilter, items, map).
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SessionCapabilitySystems;

/// Session router, capabilities, and pause projection. Independent of world
/// construction.
pub struct SessionPlugin;

impl Plugin for SessionPlugin {
	fn build(&self, app: &mut App) {
		ControllerPipeline::configure(app);
		app.add_message::<RoutingEvent>()
			.add_message::<WorldPlaySessionEnded>()
			.add_message::<ActorRetired>()
			.init_resource::<InputRouter>()
			.add_systems(Last, publish_routing_events)
			.init_resource::<WorldGameplayEnabled>()
			.init_resource::<SessionCapabilities>()
			.configure_sets(
				Update,
				SessionCapabilitySystems
					.after(ControllerPipeline::Bind)
					.before(ControllerPipeline::ResetMotorRequests),
			)
			.configure_sets(
				PostUpdate,
				SessionLifecycleSystems::Coordinate.after(SessionLifecycleSystems::Notify),
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
	fn session_capabilities_run_after_bind_before_reset() {
		let mut app = App::new();
		app.add_plugins(SessionPlugin).init_resource::<StageOrder>().add_systems(
			Update,
			(
				note("bind").in_set(ControllerPipeline::Bind),
				note("session").in_set(SessionCapabilitySystems),
				note("reset").in_set(ControllerPipeline::ResetMotorRequests),
			),
		);
		app.update();
		assert_eq!(app.world().resource::<StageOrder>().0, ["bind", "session", "reset"]);
	}

	#[test]
	fn plugin_installs_router_and_session() {
		let mut app = App::new();
		app.add_plugins(SessionPlugin);
		assert!(app.world().get_resource::<InputRouter>().is_some());
		assert!(app.world().get_resource::<SessionCapabilities>().is_some());
		assert!(app.world().get_resource::<Messages<WorldPlaySessionEnded>>().is_some());
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
		app.add_plugins(SessionPlugin)
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
