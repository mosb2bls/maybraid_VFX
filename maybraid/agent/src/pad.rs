//! Device-agnostic pad intent the world copies onto the virtual-pad overlay.

use serde::{Deserialize, Serialize};

use crate::command::AgentButton;

/// One frame of injected analog + digital. Applied after HID producers.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PadIntent {
	pub move_stick: [f32; 2],
	pub look_stick: [f32; 2],
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub buttons: Vec<AgentButton>,
}

impl PadIntent {
	pub fn from_hold(
		r#move: Option<[f32; 2]>,
		look: Option<[f32; 2]>,
		focus: Option<f32>,
		fire: Option<f32>,
		buttons: &[AgentButton],
	) -> Self {
		let mut intent = Self {
			move_stick: clamp_stick(r#move.unwrap_or([0.0, 0.0])),
			look_stick: clamp_stick(look.unwrap_or([0.0, 0.0])),
			trigger_focus: focus.unwrap_or(0.0).clamp(0.0, 1.0),
			trigger_fire: fire.unwrap_or(0.0).clamp(0.0, 1.0),
			buttons: buttons.to_vec(),
		};
		if focus.unwrap_or(0.0) > 0.0 && !intent.buttons.contains(&AgentButton::Focus) {
			intent.buttons.push(AgentButton::Focus);
		}
		if fire.unwrap_or(0.0) > 0.0 && !intent.buttons.contains(&AgentButton::Fire) {
			intent.buttons.push(AgentButton::Fire);
		}
		intent
	}

	pub fn press(button: AgentButton) -> Self {
		let mut intent = Self::default();
		intent.hold_button(button);
		intent
	}

	pub fn hold_button(&mut self, button: AgentButton) {
		if !self.buttons.contains(&button) {
			self.buttons.push(button);
		}
		match button {
			AgentButton::Focus => self.trigger_focus = self.trigger_focus.max(1.0),
			AgentButton::Fire => self.trigger_fire = self.trigger_fire.max(1.0),
			_ => {}
		}
	}

	pub fn is_idle(&self) -> bool {
		self.move_stick == [0.0, 0.0]
			&& self.look_stick == [0.0, 0.0]
			&& self.trigger_focus == 0.0
			&& self.trigger_fire == 0.0
			&& self.buttons.is_empty()
	}
}

pub fn clamp_stick(value: [f32; 2]) -> [f32; 2] {
	let len = (value[0] * value[0] + value[1] * value[1]).sqrt();
	if len <= 1.0 {
		value
	} else {
		[value[0] / len, value[1] / len]
	}
}

/// World XZ delta → camera-relative move stick (same basis as world movement).
///
/// `yaw` is the follow-camera yaw. Forward is `yaw * -Z`.
pub fn camera_relative_move(from: [f32; 2], to: [f32; 2], yaw: f32) -> [f32; 2] {
	let dx = to[0] - from[0];
	let dz = to[1] - from[1];
	let dist = (dx * dx + dz * dz).sqrt();
	if dist < 1e-5 {
		return [0.0, 0.0];
	}
	let nx = dx / dist;
	let nz = dz / dist;
	let (sin, cos) = yaw.sin_cos();
	clamp_stick([cos * nx - sin * nz, -sin * nx - cos * nz])
}

pub fn xz_distance(a: [f32; 2], b: [f32; 2]) -> f32 {
	let dx = a[0] - b[0];
	let dz = a[1] - b[1];
	(dx * dx + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn walk_forward_is_positive_y_stick() {
		// yaw = 0: forward = (0, 0, -1). Target north of player (smaller z).
		let stick = camera_relative_move([0.0, 0.0], [0.0, -10.0], 0.0);
		assert!((stick[0]).abs() < 1e-5, "{stick:?}");
		assert!((stick[1] - 1.0).abs() < 1e-5, "{stick:?}");
	}

	#[test]
	fn walk_right_is_positive_x_stick() {
		let stick = camera_relative_move([0.0, 0.0], [10.0, 0.0], 0.0);
		assert!((stick[0] - 1.0).abs() < 1e-5, "{stick:?}");
		assert!((stick[1]).abs() < 1e-5, "{stick:?}");
	}

	#[test]
	fn hold_clamps_overlong_stick() {
		let intent = PadIntent::from_hold(Some([3.0, 4.0]), None, None, None, &[]);
		assert!((intent.move_stick[0] - 0.6).abs() < 1e-5);
		assert!((intent.move_stick[1] - 0.8).abs() < 1e-5);
	}
}
