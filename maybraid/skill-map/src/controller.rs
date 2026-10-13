//! Stick-flick detector. Either stick while **RB** is held; a hold is not a flick.
//! Emits [`SkillMapFlick`].

use bevy::prelude::*;
use maybraid_input::analog::Deadzone;
use maybraid_input::produce::gamepad::GamepadAxes;
use maybraid_input::{PadButton, VirtualPad, VirtualPadConfig};

use crate::SkillMapEnabled;

/// Rest nest. Below this the stick has come home.
pub const FLICK_REST: f32 = 0.28;
/// Peak must clear this or it was a nudge.
pub const FLICK_MIN_PEAK: f32 = 0.28;
/// Faster than a hold, slower than a one-frame tap.
pub const FLICK_MIN_SECS: f32 = 0.02;
pub const FLICK_MAX_SECS: f32 = 0.32;

/// One completed flick. Direction is the stick throw; length is how hard (0..1+).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SkillMapFlick(pub Vec2);

#[derive(Clone, Debug, Default)]
struct FlickLane {
	active: bool,
	t0: f32,
	peak: Vec2,
}

impl FlickLane {
	fn reset(&mut self) {
		self.active = false;
		self.peak = Vec2::ZERO;
	}

	fn sample(&mut self, t: f32, stick: Vec2) -> Option<Vec2> {
		let mag = stick.length();
		if !self.active {
			if mag >= FLICK_REST {
				self.active = true;
				self.t0 = t;
				self.peak = stick;
			}
			return None;
		}
		if mag > self.peak.length() {
			self.peak = stick;
		}
		let elapsed = t - self.t0;
		if elapsed > FLICK_MAX_SECS {
			if mag < FLICK_REST {
				self.reset();
			}
			return None;
		}
		if mag >= FLICK_REST {
			return None;
		}
		self.active = false;
		let peak = self.peak;
		self.peak = Vec2::ZERO;
		if elapsed >= FLICK_MIN_SECS && peak.length() >= FLICK_MIN_PEAK {
			Some(peak)
		} else {
			None
		}
	}
}

/// How the map consumes stick input.
///
/// Standard keeps RB + either-stick flicks. Tanz disables that producer so
/// committed Tanz gestures can publish [`SkillMapFlick`] instead.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillMapSteerPolicy {
	#[default]
	Standard,
	Tanz,
}

/// True while the skill-map steer modifier is down (Xbox **RB** / keyboard **V**)
/// and the Tanzer bumper chord is not held.
pub fn skill_map_steer_held(pad: &VirtualPad) -> bool {
	pad.pressed(PadButton::BumperFire) && !pad.bumper_chord_held()
}

/// Watches move and look sticks. One flick per sample, stronger stick wins if both land.
#[derive(Resource, Clone, Debug, Default)]
pub struct SkillMapController {
	move_lane: FlickLane,
	look_lane: FlickLane,
}

impl SkillMapController {
	pub fn reset(&mut self) {
		self.move_lane.reset();
		self.look_lane.reset();
	}

	pub fn sample(&mut self, t: f32, move_stick: Vec2, look_stick: Vec2) -> Option<Vec2> {
		let moved = self.move_lane.sample(t, move_stick);
		let looked = self.look_lane.sample(t, look_stick);
		match (moved, looked) {
			(Some(a), Some(b)) if b.length_squared() > a.length_squared() => Some(b),
			(Some(a), _) => Some(a),
			(None, Some(b)) => Some(b),
			(None, None) => None,
		}
	}
}

pub fn detect_skill_map_flicks(
	time: Res<Time>,
	pad: Option<Res<VirtualPad>>,
	config: Option<Res<VirtualPadConfig>>,
	gamepads: Query<&Gamepad>,
	enabled: Res<SkillMapEnabled>,
	policy: Option<Res<SkillMapSteerPolicy>>,
	mut controller: ResMut<SkillMapController>,
	mut flicks: MessageWriter<SkillMapFlick>,
) {
	if !enabled.0 || policy.is_some_and(|policy| *policy == SkillMapSteerPolicy::Tanz) {
		controller.reset();
		return;
	}
	let Some(pad) = pad else {
		return;
	};
	if !skill_map_steer_held(&pad) {
		controller.reset();
		return;
	}
	let deadzone = config.map(|c| c.stick_deadzone).unwrap_or(Deadzone(0.15));
	let mut look = Vec2::ZERO;
	for gamepad in &gamepads {
		look += GamepadAxes::look_stick(gamepad, deadzone);
	}
	let look = look.clamp_length_max(1.0);
	if let Some(flick) = controller.sample(time.elapsed_secs(), pad.move_stick, look) {
		flicks.write(SkillMapFlick(flick));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn throw(
		controller: &mut SkillMapController,
		t0: f32,
		stick: Vec2,
		on_move: bool,
	) -> Option<Vec2> {
		let mut last = None;
		for (i, mag) in [0.0, 0.75, 1.0, 0.4, 0.0].into_iter().enumerate() {
			let t = t0 + i as f32 * 0.04;
			let v = stick * mag;
			last = if on_move {
				controller.sample(t, v, Vec2::ZERO)
			} else {
				controller.sample(t, Vec2::ZERO, v)
			};
		}
		last
	}

	#[test]
	fn a_fast_throw_on_either_stick_is_a_flick() {
		let mut controller = SkillMapController::default();
		let moved = throw(&mut controller, 0.0, Vec2::X, true).expect("move flick");
		assert!(moved.x > 0.9);
		let looked = throw(&mut controller, 1.0, Vec2::Y, false).expect("look flick");
		assert!(looked.y > 0.9);
	}

	#[test]
	fn a_held_stick_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.3, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.35, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn rb_is_the_steer_hold() {
		let mut pad = VirtualPad::default();
		assert!(!skill_map_steer_held(&pad));
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(skill_map_steer_held(&pad));
	}

	#[test]
	fn bumper_chord_is_not_the_steer_hold() {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(!skill_map_steer_held(&pad));
	}

	#[test]
	fn a_nudge_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X * 0.15, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 0.2, Vec2::ZERO).is_none());
		assert!(controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none());
	}
}
