//! Platform HID backends that fill Bevy `Gamepad`.
//!
//! Linux / Windows keep gilrs (via `DefaultPlugins`). Apple replaces gilrs with
//! GameController.framework because IOKit enumerates Xbox pads but does not
//! deliver reports.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod gamecontroller;

/// Chain on `DefaultPlugins.set(…)` so Apple builds do not start gilrs.
///
/// ```ignore
/// App::new().add_plugins(DefaultPlugins.set(WindowPlugin { .. }).with_pad_hid())
/// ```
pub trait PadHidPlugins {
	fn with_pad_hid(self) -> PluginGroupBuilder;
}

impl PadHidPlugins for PluginGroupBuilder {
	fn with_pad_hid(self) -> PluginGroupBuilder {
		with_pad_hid(self)
	}
}

/// Disable [`bevy::gilrs::GilrsPlugin`] on Apple. No-op elsewhere.
pub fn with_pad_hid(group: PluginGroupBuilder) -> PluginGroupBuilder {
	#[cfg(any(target_os = "macos", target_os = "ios"))]
	{
		group.disable::<bevy::gilrs::GilrsPlugin>()
	}
	#[cfg(not(any(target_os = "macos", target_os = "ios")))]
	{
		group
	}
}

pub(crate) fn configure_backend(app: &mut App) {
	#[cfg(any(target_os = "macos", target_os = "ios"))]
	{
		if app.is_plugin_added::<bevy::gilrs::GilrsPlugin>() {
			warn!(
				"GilrsPlugin is still enabled on Apple; Xbox pads may connect without events. \
				 Chain `.with_pad_hid()` on DefaultPlugins."
			);
		}
		app.add_plugins(gamecontroller::GameControllerPlugin);
	}
	#[cfg(not(any(target_os = "macos", target_os = "ios")))]
	{
		let _ = app;
	}
}
