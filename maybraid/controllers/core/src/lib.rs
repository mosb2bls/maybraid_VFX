//! Targeting, routing, capture, and schedule. No game crate deps.

pub mod actor;
pub mod capture;
pub mod route;
pub mod schedule;
pub mod source;
pub mod ui;

pub use actor::ActorTarget;
pub use capture::{CaptureKind, CaptureLayer, CaptureStack};
pub use route::{RouteTarget, Routed};
pub use schedule::{ControllerPipeline, MotorRequestStage};
pub use source::InputSource;
pub use ui::UiTarget;

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;

/// Installs the pipeline graph and an empty [`CaptureStack`].
pub struct ControllersCorePlugin;

impl Plugin for ControllersCorePlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		ControllerPipeline::configure(app);
		app.init_resource::<CaptureStack>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_installs_capture_and_pipeline() {
		let mut app = App::new();
		app.add_plugins(ControllersCorePlugin);
		assert!(app.world().get_resource::<CaptureStack>().is_some());
	}
}
