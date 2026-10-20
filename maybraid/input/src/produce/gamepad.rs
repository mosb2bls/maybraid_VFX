//! `Gamepad` → analog sticks / triggers and digital [`PadButton`]s.

use bevy::prelude::*;

use crate::analog::Deadzone;
use crate::button::PadButton;
use crate::config::VirtualPadConfig;
use crate::pad::VirtualPad;

/// Named sticks first; gilrs sometimes parks unmapped axes on [`GamepadAxis::Other`].
pub struct GamepadAxes;

impl GamepadAxes {
	pub fn fallback_stick(named: Vec2, other: Vec2, deadzone: Deadzone) -> Vec2 {
		let named = deadzone.apply_vec2(named);
		if named.length_squared() > 1e-6 {
			named
		} else {
			deadzone.apply_vec2(other)
		}
	}

	pub fn other_pair(gamepad: &Gamepad, x: u8, y: u8) -> Vec2 {
		Vec2::new(
			gamepad.get(GamepadAxis::Other(x)).unwrap_or(0.0),
			gamepad.get(GamepadAxis::Other(y)).unwrap_or(0.0),
		)
	}

	pub fn move_stick(gamepad: &Gamepad, deadzone: Deadzone) -> Vec2 {
		Self::fallback_stick(gamepad.left_stick(), Self::other_pair(gamepad, 0, 1), deadzone)
	}

	pub fn look_stick(gamepad: &Gamepad, deadzone: Deadzone) -> Vec2 {
		Self::fallback_stick(gamepad.right_stick(), Self::other_pair(gamepad, 2, 3), deadzone)
	}

	pub fn analog_dump(gamepad: &Gamepad) -> String {
		let mut parts = Vec::new();
		for input in gamepad.get_analog_axes() {
			let Some(value) = gamepad.get(*input) else {
				continue;
			};
			if value.abs() < 0.02 {
				continue;
			}
			parts.push(format!("{input:?}={value:.2}"));
		}
		parts.join(" ")
	}
}

pub fn produce_gamepad(
	gamepads: Query<&Gamepad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	mut pad: ResMut<VirtualPad>,
) {
	let dt = time.delta_secs();
	let press = config.trigger_press_threshold;
	for gamepad in &gamepads {
		pad.add_move(GamepadAxes::move_stick(gamepad, config.stick_deadzone));

		let look = GamepadAxes::look_stick(gamepad, config.stick_deadzone);
		pad.add_look_throw(look);
		pad.add_look(Vec2::new(look.x, -look.y) * config.gamepad_look_pixels_per_sec * dt);

		let focus = gamepad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0).clamp(0.0, 1.0);
		let fire = gamepad.get(GamepadButton::RightTrigger2).unwrap_or(0.0).clamp(0.0, 1.0);
		pad.max_triggers(focus, fire);

		pad.add_dpad(config.stick_deadzone.apply_vec2(gamepad.dpad()));

		for button in PadButton::ALL {
			let Some(hid) = button.gamepad() else {
				continue;
			};
			let analog = gamepad.get(hid).unwrap_or(0.0);
			if gamepad.pressed(hid) || analog >= press {
				pad.hold_digital(button);
			}
		}

		for button in gamepad.get_pressed().copied() {
			if let Some(mapped) = PadButton::from_gamepad(button) {
				pad.hold_digital(mapped);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn named_stick_wins_over_other() -> anyhow::Result<()> {
		let zone = Deadzone(0.15);
		let out = GamepadAxes::fallback_stick(Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0), zone);
		assert!((out.x - 1.0).abs() < 1e-5);
		assert!(out.y.abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn other_stick_used_when_named_is_dead() -> anyhow::Result<()> {
		let zone = Deadzone(0.15);
		let out = GamepadAxes::fallback_stick(Vec2::ZERO, Vec2::new(1.0, 0.0), zone);
		assert!((out.x - 1.0).abs() < 1e-5);
		Ok(())
	}
}
