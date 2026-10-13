//! Temporal Tanz recognizer and the committed [`TanzInput`] message.
//!
//! This module does not know about Skill Map state, Tanz equipment, or
//! animation. It turns normalized left-stick / A / B samples into discrete
//! gestures.

use std::collections::VecDeque;

use bevy::prelude::*;
use maybraid_input::{PadButton, VirtualPad};

/// Observation window used for mean / coherence integrals.
pub const TANZ_WINDOW_SECS: f32 = 0.250;
/// Minimum time a qualifying direction must be held before it commits.
pub const TANZ_MIN_COMMIT_SECS: f32 = 0.120;
pub const TANZ_MIN_MAGNITUDE: f32 = 0.55;
pub const TANZ_MIN_COHERENCE: f32 = 0.80;
pub const TANZ_REARM: f32 = 0.25;
pub const TANZ_B_HOLD_SECS: f32 = 0.300;
pub const TANZ_TOGGLE_SECS: f32 = 0.300;
pub const TANZ_DIR_CHANGE: f32 = std::f32::consts::FRAC_PI_4;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TanzButton {
	#[default]
	None,
	APress,
	BPress,
	BHold,
}

/// One committed gesture. Independently readable by Skill Map navigation and
/// Tanz matching; neither consumer should remove it for the other.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct TanzInput {
	pub user: Entity,
	pub direction: Vec2,
	pub magnitude: f32,
	pub coherence: f32,
	pub button: TanzButton,
}

impl TanzInput {
	pub fn stick(self) -> Vec2 {
		self.direction * self.magnitude
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommittedGesture {
	pub direction: Vec2,
	pub magnitude: f32,
	pub coherence: f32,
	pub button: TanzButton,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TanzRecognizerConfig {
	pub window: f32,
	pub min_commit: f32,
	pub min_magnitude: f32,
	pub min_coherence: f32,
	pub rearm: f32,
	pub b_hold: f32,
	pub dir_change: f32,
}

impl Default for TanzRecognizerConfig {
	fn default() -> Self {
		Self {
			window: TANZ_WINDOW_SECS,
			min_commit: TANZ_MIN_COMMIT_SECS,
			min_magnitude: TANZ_MIN_MAGNITUDE,
			min_coherence: TANZ_MIN_COHERENCE,
			rearm: TANZ_REARM,
			b_hold: TANZ_B_HOLD_SECS,
			dir_change: TANZ_DIR_CHANGE,
		}
	}
}

#[derive(Clone, Copy, Debug)]
struct StickSample {
	stick: Vec2,
	dt: f32,
}

/// Rolling left-stick history plus directional / button latch state.
#[derive(Clone, Debug)]
pub struct TanzRecognizer {
	samples: VecDeque<StickSample>,
	candidate: Option<Vec2>,
	committed: Option<Vec2>,
	sustain_secs: f32,
	b_hold_emitted: bool,
}

impl Default for TanzRecognizer {
	fn default() -> Self {
		Self {
			samples: VecDeque::new(),
			candidate: None,
			committed: None,
			sustain_secs: 0.0,
			b_hold_emitted: false,
		}
	}
}

impl TanzRecognizer {
	pub fn reset(&mut self) {
		self.samples.clear();
		self.candidate = None;
		self.committed = None;
		self.sustain_secs = 0.0;
		self.b_hold_emitted = false;
	}

	/// Consume one frame of left-stick + face-button state.
	///
	/// A and B rising edges, and the first crossing of the B-hold threshold,
	/// each emit. A sustained direction emits once until rearm (neutral or a
	/// new sustained direction). When a direction and a button commit on the
	/// same frame, one combined gesture is returned.
	pub fn sample(
		&mut self,
		dt: f32,
		stick: Vec2,
		pad: &VirtualPad,
		cfg: &TanzRecognizerConfig,
	) -> Option<CommittedGesture> {
		let dt = dt.max(0.0);
		if !pad.pressed(PadButton::B) {
			self.b_hold_emitted = false;
		}

		let stick_mag = stick.length();
		let stick_dir = if stick_mag > 1e-6 { stick / stick_mag } else { Vec2::ZERO };
		if stick_mag < cfg.rearm {
			self.samples.clear();
			self.candidate = None;
			self.committed = None;
			self.sustain_secs = 0.0;
		} else {
			if let Some(candidate) = self.candidate {
				if stick_dir != Vec2::ZERO && angle_between(candidate, stick_dir) >= cfg.dir_change
				{
					self.samples.clear();
					self.candidate = Some(stick_dir);
					self.sustain_secs = 0.0;
				}
			} else if stick_dir != Vec2::ZERO {
				self.candidate = Some(stick_dir);
				self.sustain_secs = 0.0;
			}
			self.samples.push_back(StickSample { stick, dt });
			trim_window(&mut self.samples, cfg.window);
			self.sustain_secs += dt;
		}

		let (mean, mean_mag, coherence) = window_stats(&self.samples);
		let mean_dir = if mean.length_squared() > 1e-8 { mean.normalize() } else { Vec2::ZERO };

		let mut direction_commit = false;
		if stick_mag >= cfg.min_magnitude
			&& coherence >= cfg.min_coherence
			&& self.sustain_secs >= cfg.min_commit
		{
			let next = if mean_dir != Vec2::ZERO { mean_dir } else { stick_dir };
			if next != Vec2::ZERO
				&& self
					.committed
					.is_none_or(|committed| angle_between(committed, next) >= cfg.dir_change)
			{
				self.committed = Some(next);
				direction_commit = true;
			}
		}

		let button = self.button_edge(pad, cfg);
		if button.is_none() && !direction_commit {
			return None;
		}

		let direction = self
			.committed
			.or(self.candidate)
			.filter(|dir| *dir != Vec2::ZERO)
			.unwrap_or(Vec2::ZERO);
		let magnitude = if direction == Vec2::ZERO { 0.0 } else { mean_mag.max(stick_mag) };
		Some(CommittedGesture {
			direction,
			magnitude,
			coherence,
			button: button.unwrap_or(TanzButton::None),
		})
	}

	fn button_edge(&mut self, pad: &VirtualPad, cfg: &TanzRecognizerConfig) -> Option<TanzButton> {
		if pad.just_pressed(PadButton::A) {
			return Some(TanzButton::APress);
		}
		if pad.just_pressed(PadButton::B) {
			return Some(TanzButton::BPress);
		}
		if pad.pressed(PadButton::B)
			&& pad.hold_secs(PadButton::B) >= cfg.b_hold
			&& !self.b_hold_emitted
		{
			self.b_hold_emitted = true;
			return Some(TanzButton::BHold);
		}
		None
	}
}

/// Latch for the LB+RB mode chord: one toggle per hold, then both must release.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeToggle {
	pub held_secs: f32,
	pub fired: bool,
}

impl ModeToggle {
	pub fn reset(&mut self) {
		self.held_secs = 0.0;
		self.fired = false;
	}

	pub fn sample(&mut self, dt: f32, chord: bool, threshold: f32) -> bool {
		if !chord {
			self.reset();
			return false;
		}
		self.held_secs += dt.max(0.0);
		if !self.fired && self.held_secs >= threshold {
			self.fired = true;
			return true;
		}
		false
	}
}

fn trim_window(samples: &mut VecDeque<StickSample>, window: f32) {
	let mut span: f32 = samples.iter().map(|sample| sample.dt).sum();
	while span > window && samples.len() > 1 {
		if let Some(front) = samples.pop_front() {
			span -= front.dt;
		} else {
			break;
		}
	}
}

fn window_stats(samples: &VecDeque<StickSample>) -> (Vec2, f32, f32) {
	let mut acc = Vec2::ZERO;
	let mut mag_acc = 0.0;
	let mut t = 0.0;
	for sample in samples {
		acc += sample.stick * sample.dt;
		mag_acc += sample.stick.length() * sample.dt;
		t += sample.dt;
	}
	if t <= 1e-8 {
		return (Vec2::ZERO, 0.0, 0.0);
	}
	let coherence = if mag_acc <= 1e-8 { 0.0 } else { acc.length() / mag_acc };
	(acc / t, mag_acc / t, coherence)
}

fn angle_between(a: Vec2, b: Vec2) -> f32 {
	if a.length_squared() <= 1e-8 || b.length_squared() <= 1e-8 {
		return 0.0;
	}
	a.normalize().dot(b.normalize()).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
	use super::*;

	fn pad_stick(stick: Vec2) -> VirtualPad {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(stick);
		pad.finish_digital();
		pad
	}

	fn pad_stick_button(stick: Vec2, button: PadButton) -> VirtualPad {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(stick);
		pad.hold_digital(button);
		pad.finish_digital();
		pad
	}

	fn hold_left(recognizer: &mut TanzRecognizer, dt: f32, frames: usize) -> Vec<CommittedGesture> {
		drive(recognizer, dt, frames, Vec2::NEG_X, &pad_stick(Vec2::NEG_X))
	}

	fn drive(
		recognizer: &mut TanzRecognizer,
		dt: f32,
		frames: usize,
		stick: Vec2,
		pad: &VirtualPad,
	) -> Vec<CommittedGesture> {
		let cfg = TanzRecognizerConfig::default();
		let mut out = Vec::new();
		for _ in 0..frames {
			if let Some(gesture) = recognizer.sample(dt, stick, pad, &cfg) {
				out.push(gesture);
			}
		}
		out
	}

	#[test]
	fn stable_direction_commits_once() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let events = hold_left(&mut recognizer, 0.016, 20);
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].button, TanzButton::None);
		assert!(events[0].direction.x < -0.9);
		assert!(events[0].magnitude >= TANZ_MIN_MAGNITUDE);
		Ok(())
	}

	#[test]
	fn small_stick_does_not_commit() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let pad = pad_stick(Vec2::NEG_X * 0.3);
		let events = drive(&mut recognizer, 0.016, 30, Vec2::NEG_X * 0.3, &pad);
		assert!(events.is_empty());
		Ok(())
	}

	#[test]
	fn neutral_rearms_the_same_direction() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		let rest = pad_stick(Vec2::ZERO);
		assert!(drive(&mut recognizer, 0.016, 4, Vec2::ZERO, &rest).is_empty());
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		Ok(())
	}

	#[test]
	fn sustained_direction_change_commits_without_neutral() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		let pad = pad_stick(Vec2::Y);
		let events = drive(&mut recognizer, 0.016, 12, Vec2::Y, &pad);
		assert_eq!(events.len(), 1);
		assert!(events[0].direction.y > 0.9);
		assert_eq!(events[0].button, TanzButton::None);
		Ok(())
	}

	#[test]
	fn a_press_uses_the_held_direction() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		let mut pad = pad_stick_button(Vec2::NEG_X, PadButton::A);
		pad.tick_holds(0.0);
		let events = drive(&mut recognizer, 0.016, 1, Vec2::NEG_X, &pad);
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].button, TanzButton::APress);
		assert!(events[0].direction.x < -0.9);
		pad.begin_frame();
		pad.add_move(Vec2::NEG_X);
		pad.hold_digital(PadButton::A);
		pad.finish_digital();
		assert!(drive(&mut recognizer, 0.016, 4, Vec2::NEG_X, &pad).is_empty());
		Ok(())
	}

	#[test]
	fn b_press_and_b_hold_are_distinct() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		let mut pad = pad_stick_button(Vec2::ZERO, PadButton::B);
		let press = recognizer.sample(0.016, Vec2::ZERO, &pad, &cfg);
		assert_eq!(press.map(|gesture| gesture.button), Some(TanzButton::BPress));

		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		pad.finish_digital();
		pad.tick_holds(0.16);
		assert!(recognizer.sample(0.16, Vec2::ZERO, &pad, &cfg).is_none());

		pad.begin_frame();
		pad.hold_digital(PadButton::B);
		pad.finish_digital();
		pad.tick_holds(0.16);
		let hold = recognizer.sample(0.16, Vec2::ZERO, &pad, &cfg);
		assert_eq!(hold.map(|gesture| gesture.button), Some(TanzButton::BHold));
		assert!(recognizer.sample(0.16, Vec2::ZERO, &pad, &cfg).is_none());
		Ok(())
	}

	#[test]
	fn coherence_rejects_alternating_movement() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		let left = pad_stick(Vec2::NEG_X);
		let right = pad_stick(Vec2::X);
		let mut events = Vec::new();
		for i in 0..20 {
			let (stick, pad) = if i % 2 == 0 { (Vec2::NEG_X, &left) } else { (Vec2::X, &right) };
			if let Some(gesture) = recognizer.sample(0.016, stick, pad, &cfg) {
				events.push(gesture);
			}
		}
		assert!(events.is_empty());
		Ok(())
	}

	#[test]
	fn frame_rate_does_not_change_commitment() -> anyhow::Result<()> {
		let cfg = TanzRecognizerConfig::default();
		let pad = pad_stick(Vec2::Y);
		let mut slow = TanzRecognizer::default();
		let mut fast = TanzRecognizer::default();
		let mut slow_events = 0;
		let mut fast_events = 0;
		let mut t = 0.0;
		while t < 0.20 {
			if slow.sample(1.0 / 30.0, Vec2::Y, &pad, &cfg).is_some() {
				slow_events += 1;
			}
			t += 1.0 / 30.0;
		}
		t = 0.0;
		while t < 0.20 {
			if fast.sample(1.0 / 120.0, Vec2::Y, &pad, &cfg).is_some() {
				fast_events += 1;
			}
			t += 1.0 / 120.0;
		}
		assert_eq!(slow_events, 1);
		assert_eq!(fast_events, 1);
		Ok(())
	}

	#[test]
	fn simultaneous_button_and_direction_do_not_duplicate() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		let rest = pad_stick(Vec2::Y);
		for _ in 0..7 {
			assert!(recognizer.sample(0.016, Vec2::Y, &rest, &cfg).is_none());
		}
		let mut press = pad_stick_button(Vec2::Y, PadButton::A);
		let first = recognizer.sample(0.016, Vec2::Y, &press, &cfg);
		assert_eq!(first.map(|gesture| gesture.button), Some(TanzButton::APress));
		assert!(first.is_some_and(|gesture| gesture.direction.y > 0.9));
		press.begin_frame();
		press.add_move(Vec2::Y);
		press.hold_digital(PadButton::A);
		press.finish_digital();
		assert!(recognizer.sample(0.016, Vec2::Y, &press, &cfg).is_none());
		Ok(())
	}

	#[test]
	fn mode_toggle_fires_once_until_release() -> anyhow::Result<()> {
		let mut toggle = ModeToggle::default();
		assert!(!toggle.sample(0.16, true, TANZ_TOGGLE_SECS));
		assert!(toggle.sample(0.16, true, TANZ_TOGGLE_SECS));
		assert!(!toggle.sample(0.16, true, TANZ_TOGGLE_SECS));
		assert!(!toggle.sample(0.16, false, TANZ_TOGGLE_SECS));
		assert!(toggle.sample(0.32, true, TANZ_TOGGLE_SECS));
		Ok(())
	}
}
