//! Targeting, routing, capture, and schedule. No game crate deps.

pub mod actor;
pub mod capture;
pub mod route;
pub mod router;
pub mod schedule;
pub mod source;
pub mod ui;

pub use actor::ActorTarget;
pub use capture::{CaptureKind, CaptureLayer, CaptureStack, CaptureToken, ScopeOwner};
pub use route::{RouteTarget, Routed};
pub use router::{
	GestureEffect, HoldEffect, InputRouter, RoutingEvent, RoutingKind, SourceAssignment,
	Suspension, SuspensionToken,
};
pub use schedule::ControllerPipeline;
pub use source::InputSource;
pub use ui::UiTarget;

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;

/// Installs the pipeline graph and an empty [`InputRouter`].
pub struct ControllersCorePlugin;

impl Plugin for ControllersCorePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		ControllerPipeline::configure(app);
		app.init_resource::<InputRouter>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_installs_router_and_pipeline() {
		let mut app = App::new();
		app.add_plugins(ControllersCorePlugin);
		assert!(app.world().get_resource::<InputRouter>().is_some());
	}
}
