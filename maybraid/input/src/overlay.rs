//! Optional analog / digital overlay applied after HID producers.
//!
//! Insert [`PadOverlay`] only when an injector is active. [`apply_pad_overlay`]
//! is a no-op when the resource is missing.

use bevy::prelude::*;

use crate::button::{PadButton, PAD_BUTTON_COUNT};
use crate::pad::VirtualPad;

/// Extra stick / trigger / button contribution for this frame.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PadOverlay {
	pub move_stick: Vec2,
	pub look_stick: Vec2,
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub buttons: [bool; PAD_BUTTON_COUNT],
}

impl Default for PadOverlay {
	fn default() -> Self {
		Self {
			move_stick: Vec2::ZERO,
			look_stick: Vec2::ZERO,
			trigger_focus: 0.0,
			trigger_fire: 0.0,
			buttons: [false; PAD_BUTTON_COUNT],
		}
	}
}

impl PadOverlay {
	pub fn clear(&mut self) {
		*self = Self::default();
	}

	pub fn hold(&mut self, button: PadButton) {
		self.buttons[button.index()] = true;
	}

	pub fn apply(&self, pad: &mut VirtualPad) {
		pad.add_move(self.move_stick);
		pad.add_look(self.look_stick);
		pad.max_triggers(self.trigger_focus, self.trigger_fire);
		for button in PadButton::ALL {
			if self.buttons[button.index()] {
				pad.hold_digital(button);
			}
		}
	}
}

pub fn apply_pad_overlay(mut pad: ResMut<VirtualPad>, overlay: Option<Res<PadOverlay>>) {
	let Some(overlay) = overlay else {
		return;
	};
	overlay.apply(&mut pad);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn overlay_unions_with_existing_stick() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(0.2, 0.0));
		let mut overlay = PadOverlay::default();
		overlay.move_stick = Vec2::new(0.0, 1.0);
		overlay.hold(PadButton::A);
		overlay.apply(&mut pad);
		pad.finish_digital();
		assert!((pad.move_stick - Vec2::new(0.2, 1.0)).length() < 1e-5);
		assert!(pad.just_pressed(PadButton::A));
		Ok(())
	}

	#[test]
	fn missing_overlay_is_a_noop() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<VirtualPad>().add_systems(Update, apply_pad_overlay);
		app.world_mut().resource_mut::<VirtualPad>().begin_frame();
		app.world_mut().resource_mut::<VirtualPad>().add_move(Vec2::Y);
		app.update();
		assert_eq!(app.world().resource::<VirtualPad>().move_stick, Vec2::Y);
		Ok(())
	}
}
