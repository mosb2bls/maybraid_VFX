//! Physical keys → move / dpad / default face bindings, plus the key overlay.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::button::{ButtonPhase, ButtonStroke, PadButton};
use crate::pad::VirtualPad;

pub fn produce_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut key_reader: MessageReader<KeyboardInput>,
	mut pad: ResMut<VirtualPad>,
) {
	let mut move_stick = Vec2::ZERO;
	if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
		move_stick.y += 1.0;
	}
	if keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
		move_stick.y -= 1.0;
	}
	if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
		move_stick.x += 1.0;
	}
	if keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
		move_stick.x -= 1.0;
	}
	pad.add_move(move_stick.clamp_length_max(1.0));

	let mut dpad = Vec2::ZERO;
	if keyboard.pressed(KeyCode::ArrowUp) {
		dpad.y += 1.0;
		pad.hold_digital(PadButton::DpadUp);
	}
	if keyboard.pressed(KeyCode::ArrowDown) {
		dpad.y -= 1.0;
		pad.hold_digital(PadButton::DpadDown);
	}
	if keyboard.pressed(KeyCode::ArrowRight) {
		dpad.x += 1.0;
		pad.hold_digital(PadButton::DpadRight);
	}
	if keyboard.pressed(KeyCode::ArrowLeft) {
		dpad.x -= 1.0;
		pad.hold_digital(PadButton::DpadLeft);
	}
	pad.add_dpad(dpad);

	if keyboard.pressed(KeyCode::Space) {
		pad.hold_digital(PadButton::A);
	}
	if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
		pad.hold_digital(PadButton::StickClickMove);
	}
	if keyboard.pressed(KeyCode::Escape) || keyboard.pressed(KeyCode::KeyB) {
		pad.hold_digital(PadButton::B);
	}
	if keyboard.pressed(KeyCode::Enter) {
		pad.hold_digital(PadButton::Start);
	}
	if keyboard.pressed(KeyCode::Tab) {
		pad.hold_digital(PadButton::Select);
	}
	if keyboard.pressed(KeyCode::KeyC) {
		pad.hold_digital(PadButton::BumperFocus);
	}
	// V is RB: hold to steer the skill map, then flick a stick.
	if keyboard.pressed(KeyCode::KeyV) {
		pad.hold_digital(PadButton::BumperFire);
	}
	if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
		pad.hold_digital(PadButton::X);
	}
	if keyboard.pressed(KeyCode::KeyY) {
		pad.hold_digital(PadButton::Y);
	}
	// Brackets cycle the active equipped queue in character intent production.

	pad.keys = keyboard.clone();
	for event in key_reader.read() {
		pad.key_events.push(ButtonStroke {
			button: event.key_code,
			phase: ButtonPhase::from_button_state(event.state),
			repeat: event.repeat,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn shift_holds_l3() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<VirtualPad>()
			.add_message::<KeyboardInput>()
			.add_systems(Update, produce_keyboard);
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ShiftLeft);
		app.update();
		assert!(app.world().resource::<VirtualPad>().digital_held(PadButton::StickClickMove));

		app.world_mut()
			.resource_mut::<ButtonInput<KeyCode>>()
			.release(KeyCode::ShiftLeft);
		app.world_mut()
			.resource_mut::<ButtonInput<KeyCode>>()
			.press(KeyCode::ShiftRight);
		app.update();
		assert!(app.world().resource::<VirtualPad>().digital_held(PadButton::StickClickMove));
		Ok(())
	}

	#[test]
	fn wasd_builds_unit_move() -> anyhow::Result<()> {
		let mut keyboard = ButtonInput::<KeyCode>::default();
		keyboard.press(KeyCode::KeyW);
		keyboard.press(KeyCode::KeyD);
		let mut move_stick = Vec2::ZERO;
		if keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
			move_stick.y += 1.0;
		}
		if keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
			move_stick.x += 1.0;
		}
		let move_stick = move_stick.clamp_length_max(1.0);
		assert!((move_stick.length() - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn y_holds_swap_active() {
		let mut pad = VirtualPad::default();
		let mut keyboard = ButtonInput::<KeyCode>::default();
		keyboard.press(KeyCode::KeyY);
		if keyboard.pressed(KeyCode::KeyY) {
			pad.hold_digital(PadButton::Y);
		}
		assert!(pad.digital_held(PadButton::Y));
	}

	#[test]
	fn e_and_x_hold_interact() {
		for key in [KeyCode::KeyE, KeyCode::KeyX] {
			let mut pad = VirtualPad::default();
			let mut keyboard = ButtonInput::<KeyCode>::default();
			keyboard.press(key);
			if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
				pad.hold_digital(PadButton::X);
			}
			assert!(pad.digital_held(PadButton::X));
		}
	}

	#[test]
	fn v_holds_right_bumper() {
		let mut pad = VirtualPad::default();
		let mut keyboard = ButtonInput::<KeyCode>::default();
		keyboard.press(KeyCode::KeyV);
		if keyboard.pressed(KeyCode::KeyV) {
			pad.hold_digital(PadButton::BumperFire);
		}
		assert!(pad.digital_held(PadButton::BumperFire));
	}

}
