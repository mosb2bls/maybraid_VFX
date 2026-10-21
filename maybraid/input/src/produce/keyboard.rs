//! Physical keys → move / dpad / default face bindings, plus the key overlay.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::button::{ButtonPhase, ButtonStroke, PadButton};
use crate::gate::PadInputPolicy;
use crate::pad::VirtualPad;

pub fn produce_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	policy: Res<PadInputPolicy>,
	mut key_reader: MessageReader<KeyboardInput>,
	mut pad: ResMut<VirtualPad>,
) {
	if policy.keyboard_menu() {
		produce_menu_keys(&keyboard, &mut pad);
	} else {
		produce_gameplay_keys(&keyboard, &mut pad);
	}

	pad.keys = keyboard.clone();
	for event in key_reader.read() {
		pad.key_events.push(ButtonStroke {
			button: event.key_code,
			phase: ButtonPhase::from_button_state(event.state),
			repeat: event.repeat,
		});
	}
}

/// WASD walks. Arrows also walk and feed the D-Pad. Escape is the only
/// keyboard Back / stance face — **B** types a letter.
fn produce_gameplay_keys(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
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
	hold_arrows(keyboard, pad);

	if keyboard.pressed(KeyCode::Space) {
		pad.hold_digital(PadButton::A);
	}
	if keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
		pad.hold_digital(PadButton::StickClickMove);
	}
	if keyboard.pressed(KeyCode::Escape) {
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
}

/// Letter keys type. Arrows move the focused menu / keypad; Escape backs out;
/// Enter confirms. Gamepad faces still arrive through the HID producer.
fn produce_menu_keys(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
	hold_arrows(keyboard, pad);
	if keyboard.pressed(KeyCode::Escape) {
		pad.hold_digital(PadButton::B);
	}
	if keyboard.pressed(KeyCode::Enter) {
		pad.hold_digital(PadButton::Start);
	}
	if keyboard.pressed(KeyCode::KeyE) || keyboard.pressed(KeyCode::KeyX) {
		pad.hold_digital(PadButton::X);
	}
}

fn hold_arrows(keyboard: &ButtonInput<KeyCode>, pad: &mut VirtualPad) {
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
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gate::PadInputPolicy;

	fn press(app: &mut App, key: KeyCode) {
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
		app.update();
	}

	#[test]
	fn shift_holds_l3() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<VirtualPad>()
			.init_resource::<PadInputPolicy>()
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

	fn produce_app(policy: PadInputPolicy) -> App {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<VirtualPad>()
			.insert_resource(policy)
			.add_message::<KeyboardInput>()
			.add_systems(Update, produce_keyboard);
		app
	}

	#[test]
	fn escape_is_the_only_keyboard_back() {
		let mut app = produce_app(PadInputPolicy::gameplay());
		press(&mut app, KeyCode::KeyB);
		assert!(!app.world().resource::<VirtualPad>().digital_held(PadButton::B));
		press(&mut app, KeyCode::Escape);
		assert!(app.world().resource::<VirtualPad>().digital_held(PadButton::B));
	}

	#[test]
	fn menu_map_uses_arrows_not_wasd() {
		let mut app = produce_app(PadInputPolicy::menu());
		press(&mut app, KeyCode::KeyW);
		press(&mut app, KeyCode::KeyA);
		press(&mut app, KeyCode::Space);
		press(&mut app, KeyCode::KeyB);
		let pad = app.world().resource::<VirtualPad>();
		assert_eq!(pad.move_stick, Vec2::ZERO);
		assert_eq!(pad.dpad, Vec2::ZERO);
		assert!(!pad.digital_held(PadButton::A));
		assert!(!pad.digital_held(PadButton::B));

		press(&mut app, KeyCode::ArrowUp);
		press(&mut app, KeyCode::Escape);
		let pad = app.world().resource::<VirtualPad>();
		assert!(pad.digital_held(PadButton::DpadUp));
		assert!(pad.digital_held(PadButton::B));
		assert!(pad.dpad.y > 0.0);
	}

	#[test]
	fn menu_map_e_and_x_hold_drop() {
		for key in [KeyCode::KeyE, KeyCode::KeyX] {
			let mut app = produce_app(PadInputPolicy::menu());
			press(&mut app, key);
			assert!(app.world().resource::<VirtualPad>().digital_held(PadButton::X));
		}
	}
}
