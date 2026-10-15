//! Menu navigation derived from the virtual pad.

use bevy::prelude::*;

use crate::analog::Cardinal;
use crate::button::PadButton;
use crate::config::VirtualPadConfig;
use crate::pad::VirtualPad;

/// Routed menu impulse. Triggered on the focused menu entity.
#[derive(EntityEvent, Clone, Copy, Debug, PartialEq, Eq)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuNavImpulse {
	pub entity: Entity,
	pub nav: MenuNav,
}

impl MenuNavImpulse {
	pub fn new(entity: Entity, nav: MenuNav) -> Self {
		Self { entity, nav }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MenuNav {
	Select,
	Back,
	Up,
	Down,
	Left,
	Right,
}

impl MenuNav {
	pub fn from_cardinal(cardinal: Cardinal) -> Self {
		match cardinal {
			Cardinal::Up => Self::Up,
			Cardinal::Down => Self::Down,
			Cardinal::Left => Self::Left,
			Cardinal::Right => Self::Right,
		}
	}

	/// `A` or `Start` → Select, `B` → Back, dominant stick / look / dpad → arrows.
	pub fn from_pad(pad: &VirtualPad, threshold: f32) -> Vec<Self> {
		let mut events = Vec::new();
		if pad.just_pressed(PadButton::A) || pad.just_pressed(PadButton::Start) {
			events.push(Self::Select);
		}
		if pad.just_pressed(PadButton::B) {
			events.push(Self::Back);
		}
		if let Some(dir) = Self::direction_from_pad(pad, threshold) {
			events.push(dir);
		}
		events
	}

	pub fn direction_from_pad(pad: &VirtualPad, threshold: f32) -> Option<Self> {
		if pad.just_pressed(PadButton::DpadUp) {
			return Some(Self::Up);
		}
		if pad.just_pressed(PadButton::DpadDown) {
			return Some(Self::Down);
		}
		if pad.just_pressed(PadButton::DpadLeft) {
			return Some(Self::Left);
		}
		if pad.just_pressed(PadButton::DpadRight) {
			return Some(Self::Right);
		}
		Cardinal::from_stick(pad.dpad, threshold)
			.or_else(|| Cardinal::from_stick(pad.move_stick, threshold))
			.map(Self::from_cardinal)
	}
}

#[derive(Resource, Clone, Debug, Default)]
pub struct MenuNavPad {
	pub events: Vec<MenuNav>,
	held_dir: Option<MenuNav>,
	repeat_clock: f32,
	repeating: bool,
}

impl MenuNavPad {
	pub fn just_pressed(&self, nav: MenuNav) -> bool {
		self.events.contains(&nav)
	}

	pub fn derive(&mut self, pad: &VirtualPad, config: &VirtualPadConfig, dt: f32) {
		self.events.clear();
		if pad.just_pressed(PadButton::A) || pad.just_pressed(PadButton::Start) {
			self.events.push(MenuNav::Select);
		}
		if pad.just_pressed(PadButton::B) {
			self.events.push(MenuNav::Back);
		}

		let edge_dir = if pad.just_pressed(PadButton::DpadUp) {
			Some(MenuNav::Up)
		} else if pad.just_pressed(PadButton::DpadDown) {
			Some(MenuNav::Down)
		} else if pad.just_pressed(PadButton::DpadLeft) {
			Some(MenuNav::Left)
		} else if pad.just_pressed(PadButton::DpadRight) {
			Some(MenuNav::Right)
		} else {
			None
		};

		let held = Cardinal::from_stick(pad.dpad, config.menu_stick_threshold)
			.or_else(|| Cardinal::from_stick(pad.move_stick, config.menu_stick_threshold))
			.map(MenuNav::from_cardinal);

		if let Some(dir) = edge_dir {
			self.events.push(dir);
			self.held_dir = Some(dir);
			self.repeat_clock = 0.0;
			self.repeating = false;
			return;
		}

		match (held, self.held_dir) {
			(Some(dir), Some(prev)) if dir == prev => {
				self.repeat_clock += dt;
				let interval = if self.repeating {
					config.menu_repeat_rate_secs
				} else {
					config.menu_repeat_delay_secs
				};
				if self.repeat_clock >= interval {
					self.events.push(dir);
					self.repeat_clock = 0.0;
					self.repeating = true;
				}
			}
			(Some(dir), _) => {
				self.events.push(dir);
				self.held_dir = Some(dir);
				self.repeat_clock = 0.0;
				self.repeating = false;
			}
			(None, _) => {
				self.held_dir = None;
				self.repeat_clock = 0.0;
				self.repeating = false;
			}
		}
	}
}

pub fn derive_menu_nav(
	pad: Res<VirtualPad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	mut menu: ResMut<MenuNavPad>,
) {
	menu.derive(&pad, &config, time.delta_secs());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_and_start_select() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		assert_eq!(MenuNav::from_pad(&pad, 0.5), vec![MenuNav::Select]);
		Ok(())
	}

	#[test]
	fn b_is_back() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		pad.finish_digital();
		assert_eq!(MenuNav::from_pad(&pad, 0.5), vec![MenuNav::Back]);
		Ok(())
	}

	/// Gamepad left stick still navigates. Keyboard WASD does not — it never
	/// reaches the stick under [`crate::KeyboardPadMap::Menu`].
	#[test]
	fn stick_becomes_cardinal() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(0.0, 0.9));
		pad.finish_digital();
		assert_eq!(MenuNav::from_pad(&pad, 0.5), vec![MenuNav::Up]);
		Ok(())
	}

	#[test]
	fn hold_repeats_after_delay() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.add_move(Vec2::new(0.0, 1.0));
		let config = VirtualPadConfig::default();
		let mut menu = MenuNavPad::default();
		menu.derive(&pad, &config, 0.0);
		assert_eq!(menu.events, vec![MenuNav::Up]);
		menu.derive(&pad, &config, 0.1);
		assert!(menu.events.is_empty());
		menu.derive(&pad, &config, 0.3);
		assert_eq!(menu.events, vec![MenuNav::Up]);
		Ok(())
	}
}
