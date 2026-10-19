//! Per-player virtual pad snapshot.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::button::{ButtonPhase, ButtonStroke, PadButton, PAD_BUTTON_COUNT};

/// Device-agnostic pad for one player. Analog is current value; digital is
/// [`ButtonInput`] (pressed / just_pressed / just_released) plus this-frame edges.
///
/// Keyboard-attached devices also fill [`Self::keys`]. Text / IME is not stored
/// here — keep reading Bevy [`KeyboardInput`](bevy::input::keyboard::KeyboardInput).
#[derive(Resource, Clone, Debug)]
pub struct VirtualPad {
	pub move_stick: Vec2,
	pub look_stick: Vec2,
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub dpad: Vec2,
	pub buttons: ButtonInput<PadButton>,
	pub button_events: Vec<ButtonStroke<PadButton>>,
	pub button_hold_secs: [f32; PAD_BUTTON_COUNT],
	pub keys: ButtonInput<KeyCode>,
	pub key_events: Vec<ButtonStroke<KeyCode>>,
	pub key_hold_secs: HashMap<KeyCode, f32>,
	/// Working set for this frame's digital union. Cleared in [`Self::begin_frame`].
	digital_mask: u16,
	/// Buttons held through a [`Self::suppress_gameplay`] frame. They stay
	/// released until the device lets go, so lifting the gate is not a press.
	swallowed: u16,
	suppressed: bool,
}

impl Default for VirtualPad {
	fn default() -> Self {
		Self {
			move_stick: Vec2::ZERO,
			look_stick: Vec2::ZERO,
			trigger_focus: 0.0,
			trigger_fire: 0.0,
			dpad: Vec2::ZERO,
			buttons: ButtonInput::default(),
			button_events: Vec::new(),
			button_hold_secs: [0.0; PAD_BUTTON_COUNT],
			keys: ButtonInput::default(),
			key_events: Vec::new(),
			key_hold_secs: HashMap::new(),
			digital_mask: 0,
			swallowed: 0,
			suppressed: false,
		}
	}
}

impl VirtualPad {
	pub fn just_pressed(&self, button: PadButton) -> bool {
		self.buttons.just_pressed(button)
	}

	pub fn pressed(&self, button: PadButton) -> bool {
		self.buttons.pressed(button)
	}

	pub fn just_released(&self, button: PadButton) -> bool {
		self.buttons.just_released(button)
	}

	pub fn hold_secs(&self, button: PadButton) -> f32 {
		self.button_hold_secs[button.index()]
	}

	/// True while both shoulder bumpers are down (LB + RB).
	///
	/// Used as the Standard ↔ Tanzer chord so individual bumper actions stay
	/// live unless both buttons are participating.
	pub fn bumper_chord_held(&self) -> bool {
		self.pressed(PadButton::BumperFocus) && self.pressed(PadButton::BumperFire)
	}

	pub fn key_hold_secs(&self, key: KeyCode) -> f32 {
		self.key_hold_secs.get(&key).copied().unwrap_or(0.0)
	}

	pub fn begin_frame(&mut self) {
		self.move_stick = Vec2::ZERO;
		self.look_stick = Vec2::ZERO;
		self.trigger_focus = 0.0;
		self.trigger_fire = 0.0;
		self.dpad = Vec2::ZERO;
		self.buttons.clear();
		self.button_events.clear();
		self.key_events.clear();
		self.digital_mask = 0;
		self.suppressed = false;
	}

	pub fn add_move(&mut self, value: Vec2) {
		self.move_stick += value;
	}

	pub fn add_look(&mut self, value: Vec2) {
		self.look_stick += value;
	}

	pub fn add_dpad(&mut self, value: Vec2) {
		self.dpad += value;
	}

	pub fn max_triggers(&mut self, focus: f32, fire: f32) {
		self.trigger_focus = self.trigger_focus.max(focus);
		self.trigger_fire = self.trigger_fire.max(fire);
	}

	pub fn hold_digital(&mut self, button: PadButton) {
		self.digital_mask |= 1 << button.index();
	}

	pub fn digital_held(&self, button: PadButton) -> bool {
		self.digital_mask & (1 << button.index()) != 0
	}

	/// Zero analog and drop this frame's digital union (keys stay). Buttons
	/// still down when the gate lifts need a fresh press.
	pub fn suppress_gameplay(&mut self) {
		self.move_stick = Vec2::ZERO;
		self.look_stick = Vec2::ZERO;
		self.trigger_focus = 0.0;
		self.trigger_fire = 0.0;
		self.dpad = Vec2::ZERO;
		self.swallowed = self.digital_mask;
		self.digital_mask = 0;
		self.suppressed = true;
	}

	/// Apply the digital union to [`Self::buttons`] and collect edges.
	pub fn finish_digital(&mut self) {
		if !self.suppressed {
			self.swallowed &= self.digital_mask;
			self.digital_mask &= !self.swallowed;
		}
		for button in PadButton::ALL {
			if self.digital_held(button) {
				self.buttons.press(button);
			} else {
				self.buttons.release(button);
			}
		}
		for button in PadButton::ALL {
			if self.buttons.just_pressed(button) {
				self.button_events.push(ButtonStroke::pressed(button));
			}
			if self.buttons.just_released(button) {
				self.button_events.push(ButtonStroke::released(button));
			}
		}
	}

	pub fn apply_trigger_digital(&mut self, threshold: f32) {
		if self.trigger_focus >= threshold {
			self.hold_digital(PadButton::TriggerFocus);
		}
		if self.trigger_fire >= threshold {
			self.hold_digital(PadButton::TriggerFire);
		}
	}

	pub fn tick_holds(&mut self, dt: f32) {
		for button in PadButton::ALL {
			let i = button.index();
			if self.buttons.just_pressed(button) {
				self.button_hold_secs[i] = 0.0;
			} else if self.buttons.pressed(button) {
				self.button_hold_secs[i] += dt;
			} else if !self.buttons.just_released(button) {
				self.button_hold_secs[i] = 0.0;
			}
		}

		let pressed: Vec<KeyCode> = self.keys.get_pressed().copied().collect();
		for key in pressed {
			if self.keys.just_pressed(key) {
				self.key_hold_secs.insert(key, 0.0);
			} else if let Some(hold) = self.key_hold_secs.get_mut(&key) {
				*hold += dt;
			} else {
				self.key_hold_secs.insert(key, dt);
			}
		}
		self.key_hold_secs
			.retain(|key, _| self.keys.pressed(*key) || self.keys.just_released(*key));
		for key in self.keys.get_just_pressed().copied() {
			self.key_hold_secs.entry(key).or_insert(0.0);
		}
	}

	pub fn snapshot(&self) -> crate::history::PadSnapshot {
		let mut buttons_down = 0u16;
		for button in PadButton::ALL {
			if self.buttons.pressed(button) {
				buttons_down |= 1 << button.index();
			}
		}
		crate::history::PadSnapshot {
			move_stick: self.move_stick,
			look_stick: self.look_stick,
			trigger_focus: self.trigger_focus,
			trigger_fire: self.trigger_fire,
			dpad: self.dpad,
			buttons_down,
		}
	}
}

impl ButtonPhase {
	pub fn from_pad(pad: &VirtualPad, button: PadButton) -> Option<Self> {
		if pad.just_pressed(button) {
			Some(Self::Pressed)
		} else if pad.just_released(button) {
			Some(Self::Released)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn digital_edges_follow_union() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		assert!(pad.just_pressed(PadButton::A));
		assert!(pad.pressed(PadButton::A));
		assert_eq!(pad.button_events, vec![ButtonStroke::pressed(PadButton::A)]);

		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		assert!(!pad.just_pressed(PadButton::A));
		assert!(pad.pressed(PadButton::A));
		assert!(pad.button_events.is_empty());

		pad.begin_frame();
		pad.finish_digital();
		assert!(pad.just_released(PadButton::A));
		assert!(!pad.pressed(PadButton::A));
		Ok(())
	}

	fn frame(pad: &mut VirtualPad, held: &[PadButton], suppressed: bool) {
		pad.begin_frame();
		for button in held {
			pad.hold_digital(*button);
		}
		if suppressed {
			pad.suppress_gameplay();
		}
		pad.finish_digital();
	}

	#[test]
	fn lifting_suppression_under_a_held_button_is_not_a_press() {
		let mut pad = VirtualPad::default();
		frame(&mut pad, &[PadButton::B], false);
		assert!(pad.just_pressed(PadButton::B));
		frame(&mut pad, &[PadButton::B], true);
		assert!(pad.just_released(PadButton::B));
		frame(&mut pad, &[PadButton::B], false);
		assert!(!pad.just_pressed(PadButton::B));
		assert!(!pad.pressed(PadButton::B));
		assert!(pad.button_events.is_empty());

		frame(&mut pad, &[], false);
		frame(&mut pad, &[PadButton::B], false);
		assert!(pad.just_pressed(PadButton::B), "a real press after release counts");
	}

	#[test]
	fn release_during_suppression_rearms_the_button() {
		let mut pad = VirtualPad::default();
		frame(&mut pad, &[PadButton::A], true);
		frame(&mut pad, &[], true);
		frame(&mut pad, &[PadButton::A], false);
		assert!(pad.just_pressed(PadButton::A));
	}

	#[test]
	fn bumper_chord_needs_both_shoulders() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.finish_digital();
		assert!(!pad.bumper_chord_held());

		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(pad.bumper_chord_held());
		Ok(())
	}

	#[test]
	fn hold_secs_accumulate_while_down() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		pad.tick_holds(0.0);
		assert_eq!(pad.hold_secs(PadButton::A), 0.0);

		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		pad.tick_holds(0.16);
		assert!((pad.hold_secs(PadButton::A) - 0.16).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn trigger_threshold_becomes_digital() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.2, 0.8);
		pad.apply_trigger_digital(0.5);
		pad.finish_digital();
		assert!(!pad.pressed(PadButton::TriggerFocus));
		assert!(pad.just_pressed(PadButton::TriggerFire));
		Ok(())
	}
}
