//! Throttled FPS / frame-time logging for the concepts playground.
//!
//! Enable with `CROZON_FPS_DEBUG=1` and `RUST_LOG=info` (or `RUST_LOG=bevy_diagnostic=info`).

use std::time::Duration;

use bevy::{
	diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
	platform::collections::HashSet,
	prelude::*,
};

const LOG_INTERVAL_SECS: f32 = 2.0;

pub fn fps_debug_enabled() -> bool {
	std::env::var("CROZON_FPS_DEBUG").is_ok()
}

pub struct FpsDiagnosticsPlugin;

impl Plugin for FpsDiagnosticsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(FrameTimeDiagnosticsPlugin::default())
			.add_plugins(LogDiagnosticsPlugin {
				wait_duration: Duration::from_secs_f32(LOG_INTERVAL_SECS),
				filter: Some(HashSet::from_iter([
					FrameTimeDiagnosticsPlugin::FPS,
					FrameTimeDiagnosticsPlugin::FRAME_TIME,
				])),
				debug: false,
			});
	}
}
