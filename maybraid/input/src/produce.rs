//! Device producers. Run after Bevy [`InputSystems`].

use bevy::input::InputSystems;
use bevy::prelude::*;

use crate::config::VirtualPadConfig;
use crate::gate::PadGameplayEnabled;
use crate::pad::VirtualPad;

pub mod gamepad;
pub mod keyboard;
pub mod mouse;

pub fn begin_produce(mut pad: ResMut<VirtualPad>) {
	pad.begin_frame();
}

pub fn finish_produce(
	mut pad: ResMut<VirtualPad>,
	config: Res<VirtualPadConfig>,
	enabled: Res<PadGameplayEnabled>,
	time: Res<Time>,
) {
	if !enabled.is_enabled() {
		pad.suppress_gameplay();
	}
	pad.apply_trigger_digital(config.trigger_press_threshold);
	pad.finish_digital();
	pad.tick_holds(time.delta_secs());
	pad.move_stick = pad.move_stick.clamp_length_max(1.0);
	pad.dpad = pad.dpad.clamp_length_max(1.0);
}

/// Shared schedule: Bevy HID → produce → derive.
pub fn configure_produce(app: &mut App) {
	app.add_systems(
		PreUpdate,
		(
			begin_produce,
			gamepad::produce_gamepad,
			keyboard::produce_keyboard,
			mouse::produce_mouse,
			crate::overlay::apply_pad_overlay,
			finish_produce,
		)
			.chain()
			.in_set(crate::VirtualPadSystems::Produce)
			.after(InputSystems),
	);
}
