//! Temporal Tanz recognizer and the committed [`TanzInput`] message.
//!
//! This module does not know about Skill Map state, Tanz equipment, or
//! animation. It turns normalized left-stick / A / B samples into discrete
//! gestures.

use std::collections::VecDeque;

use bevy::prelude::*;
use maybraid_character_controller::{ActorControlSample, FaceButtonSample};
use maybraid_controllers_core::{
	GestureEffect, HoldEffect, InputSource, RoutingEvent, RoutingKind,
};

use crate::ControlMode;

/// Observation window used for mean / coherence integrals.
pub const TANZ_WINDOW_SECS: f32 = 0.250;
/// Minimum time a qualifying direction must be held before it commits.
pub const TANZ_MIN_COMMIT_SECS: f32 = 0.120;
pub const TANZ_MIN_MAGNITUDE: f32 = 0.55;
pub const TANZ_MIN_COHERENCE: f32 = 0.80;
pub const TANZ_REARM: f32 = 0.25;
pub const TANZ_B_HOLD_SECS: f32 = 0.300;
pub const TANZ_TOGGLE_SECS: f32 = 1.0;
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
///
/// History is scoped to one [`InputSource`]. Reassignment and disconnect
/// cancel it; suspension freezes it so a later resume cannot complete against
/// a different delivery.
#[derive(Clone, Debug)]
pub struct TanzRecognizer {
	source: Option<InputSource>,
	frozen: bool,
	samples: VecDeque<StickSample>,
	candidate: Option<Vec2>,
	committed: Option<Vec2>,
	sustain_secs: f32,
	b_hold_emitted: bool,
}

impl Default for TanzRecognizer {
	fn default() -> Self {
		Self {
			source: None,
			frozen: false,
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
		self.source = None;
		self.frozen = false;
		self.samples.clear();
		self.candidate = None;
		self.committed = None;
		self.sustain_secs = 0.0;
		self.b_hold_emitted = false;
	}

	pub const fn source(&self) -> Option<InputSource> {
		self.source
	}

	pub const fn is_frozen(&self) -> bool {
		self.frozen
	}

	pub fn has_partial(&self) -> bool {
		!self.samples.is_empty() || self.candidate.is_some() || self.committed.is_some()
	}

	/// Bind this recognizer to the source that is accumulating history.
	pub fn bind_source(&mut self, source: InputSource) {
		if self.source != Some(source) {
			self.reset();
			self.source = Some(source);
		}
	}

	/// Apply a router hold / gesture contract for this source.
	pub fn apply_routing(&mut self, event: RoutingEvent) {
		if self.source.is_some_and(|source| source != event.source()) {
			return;
		}
		if self.source.is_none() && !self.has_partial() && !self.frozen {
			return;
		}
		match event.gesture() {
			GestureEffect::Cancel => self.reset(),
			GestureEffect::Freeze => self.frozen = true,
			GestureEffect::None => {
				if matches!(event.kind(), RoutingKind::Assigned | RoutingKind::Resumed) {
					self.frozen = false;
				}
			}
		}
	}

	/// Consume one frame of left-stick + physical face-button state.
	///
	/// A and B rising edges, and the first crossing of the B-hold threshold,
	/// each emit. A sustained direction emits once until rearm (neutral or a
	/// new sustained direction). When a direction and a button commit on the
	/// same frame, one combined gesture is returned.
	pub fn sample(
		&mut self,
		dt: f32,
		stick: Vec2,
		face: TanzFaceSample,
		cfg: &TanzRecognizerConfig,
	) -> Option<CommittedGesture> {
		if self.frozen {
			return None;
		}
		let dt = dt.max(0.0);
		if !face.b().pressed() {
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

		let button = self.button_edge(face, cfg);
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

	fn button_edge(
		&mut self,
		face: TanzFaceSample,
		cfg: &TanzRecognizerConfig,
	) -> Option<TanzButton> {
		if face.a().just_pressed() {
			return Some(TanzButton::APress);
		}
		if face.b().just_pressed() {
			return Some(TanzButton::BPress);
		}
		if face.b().pressed() && face.b().hold_secs() >= cfg.b_hold && !self.b_hold_emitted {
			self.b_hold_emitted = true;
			return Some(TanzButton::BHold);
		}
		None
	}
}

/// Physical A / B sample for recognition. Distinct from semantic Jump.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TanzFaceSample {
	a: FaceButtonSample,
	b: FaceButtonSample,
}

impl TanzFaceSample {
	pub const fn new(a: FaceButtonSample, b: FaceButtonSample) -> Self {
		Self { a, b }
	}

	pub fn from_control(sample: ActorControlSample) -> Self {
		Self { a: sample.a(), b: sample.b() }
	}

	pub const fn a(self) -> FaceButtonSample {
		self.a
	}

	pub const fn b(self) -> FaceButtonSample {
		self.b
	}

	pub fn with_a_press(self) -> Self {
		Self { a: FaceButtonSample::new(true, true, false, 0.0), b: self.b }
	}

	pub fn with_b_press(self) -> Self {
		Self { a: self.a, b: FaceButtonSample::new(true, true, false, 0.0) }
	}

	pub fn with_b_hold(self, hold_secs: f32) -> Self {
		Self { a: self.a, b: FaceButtonSample::new(true, false, false, hold_secs) }
	}
}

/// Latch for the LB+RB mode chord: one toggle per hold, then both must release.
#[derive(Clone, Debug, PartialEq)]
pub struct ModeToggle {
	/// Normalized charge toward the single toggle threshold (`0` rest, `1` fire).
	charge: f32,
	/// True after this hold has already fired; both bumpers must release to rearm.
	pub fired: bool,
	frozen: bool,
}

impl Default for ModeToggle {
	fn default() -> Self {
		Self { charge: 0.0, fired: false, frozen: false }
	}
}

impl ModeToggle {
	pub fn reset_for_mode(&mut self, _mode: ControlMode) {
		self.charge = 0.0;
		self.fired = false;
		self.frozen = false;
	}

	pub const fn is_frozen(&self) -> bool {
		self.frozen
	}

	pub fn is_partial(&self) -> bool {
		self.charge > 0.0 || self.fired
	}

	/// Release or freeze a bumper hold from a [`RoutingEvent`].
	pub fn apply_hold(&mut self, effect: HoldEffect) {
		match effect {
			HoldEffect::ReleaseNeutral => self.reset_for_mode(ControlMode::Standard),
			HoldEffect::Freeze => self.frozen = true,
			HoldEffect::None => self.frozen = false,
		}
	}

	/// Displayed Standard=0 / Tanzer=1 bar.
	///
	/// Charge grows toward the threshold while the chord is held. After the
	/// latch fires and the chord is still held, charge parks at rest (`0`), so
	/// Standard reads `charge` and Tanzer reads `1 - charge`.
	pub fn bar(&self, mode: ControlMode) -> f32 {
		match mode {
			ControlMode::Standard => self.charge,
			ControlMode::Tanzer => 1.0 - self.charge,
		}
	}

	/// Advance charge for one frame. Returns `true` on the frame the mode should flip.
	pub fn sample(&mut self, dt: f32, chord: bool, charge_secs: f32, _mode: ControlMode) -> bool {
		if self.frozen {
			return false;
		}
		let dt = dt.max(0.0);
		let rate = if charge_secs > 0.0 { dt / charge_secs } else { 1.0 };
		if !chord {
			self.fired = false;
			self.charge = (self.charge - rate).max(0.0);
			return false;
		}
		if self.fired {
			self.charge = 0.0;
			return false;
		}
		self.charge = (self.charge + rate).min(1.0);
		const EPS: f32 = 1e-5;
		if self.charge + EPS >= 1.0 {
			self.charge = 1.0;
			self.fired = true;
			true
		} else {
			false
		}
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

	fn hold_left(recognizer: &mut TanzRecognizer, dt: f32, frames: usize) -> Vec<CommittedGesture> {
		drive(recognizer, dt, frames, Vec2::NEG_X, TanzFaceSample::default())
	}

	fn drive(
		recognizer: &mut TanzRecognizer,
		dt: f32,
		frames: usize,
		stick: Vec2,
		face: TanzFaceSample,
	) -> Vec<CommittedGesture> {
		let cfg = TanzRecognizerConfig::default();
		let mut out = Vec::new();
		for _ in 0..frames {
			if let Some(gesture) = recognizer.sample(dt, stick, face, &cfg) {
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
		let events =
			drive(&mut recognizer, 0.016, 30, Vec2::NEG_X * 0.3, TanzFaceSample::default());
		assert!(events.is_empty());
		Ok(())
	}

	#[test]
	fn neutral_rearms_the_same_direction() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		assert!(drive(&mut recognizer, 0.016, 4, Vec2::ZERO, TanzFaceSample::default()).is_empty());
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		Ok(())
	}

	#[test]
	fn sustained_direction_change_commits_without_neutral() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		let events = drive(&mut recognizer, 0.016, 12, Vec2::Y, TanzFaceSample::default());
		assert_eq!(events.len(), 1);
		assert!(events[0].direction.y > 0.9);
		assert_eq!(events[0].button, TanzButton::None);
		Ok(())
	}

	#[test]
	fn a_press_uses_the_held_direction() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		assert_eq!(hold_left(&mut recognizer, 0.016, 12).len(), 1);
		let events =
			drive(&mut recognizer, 0.016, 1, Vec2::NEG_X, TanzFaceSample::default().with_a_press());
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].button, TanzButton::APress);
		assert!(events[0].direction.x < -0.9);
		let held_a = TanzFaceSample::new(
			FaceButtonSample::new(true, false, false, 0.016),
			FaceButtonSample::default(),
		);
		assert!(drive(&mut recognizer, 0.016, 4, Vec2::NEG_X, held_a).is_empty());
		Ok(())
	}

	#[test]
	fn b_press_and_b_hold_are_distinct() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		let press =
			recognizer.sample(0.016, Vec2::ZERO, TanzFaceSample::default().with_b_press(), &cfg);
		assert_eq!(press.map(|gesture| gesture.button), Some(TanzButton::BPress));

		let mid = TanzFaceSample::default().with_b_hold(0.16);
		assert!(recognizer.sample(0.16, Vec2::ZERO, mid, &cfg).is_none());

		let hold =
			recognizer.sample(0.16, Vec2::ZERO, TanzFaceSample::default().with_b_hold(0.32), &cfg);
		assert_eq!(hold.map(|gesture| gesture.button), Some(TanzButton::BHold));
		assert!(recognizer
			.sample(0.16, Vec2::ZERO, TanzFaceSample::default().with_b_hold(0.48), &cfg)
			.is_none());
		Ok(())
	}

	#[test]
	fn coherence_rejects_alternating_movement() -> anyhow::Result<()> {
		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		let rest = TanzFaceSample::default();
		let mut events = Vec::new();
		for i in 0..20 {
			let stick = if i % 2 == 0 { Vec2::NEG_X } else { Vec2::X };
			if let Some(gesture) = recognizer.sample(0.016, stick, rest, &cfg) {
				events.push(gesture);
			}
		}
		assert!(events.is_empty());
		Ok(())
	}

	#[test]
	fn frame_rate_does_not_change_commitment() -> anyhow::Result<()> {
		let cfg = TanzRecognizerConfig::default();
		let rest = TanzFaceSample::default();
		let mut slow = TanzRecognizer::default();
		let mut fast = TanzRecognizer::default();
		let mut slow_events = 0;
		let mut fast_events = 0;
		let mut t = 0.0;
		while t < 0.20 {
			if slow.sample(1.0 / 30.0, Vec2::Y, rest, &cfg).is_some() {
				slow_events += 1;
			}
			t += 1.0 / 30.0;
		}
		t = 0.0;
		while t < 0.20 {
			if fast.sample(1.0 / 120.0, Vec2::Y, rest, &cfg).is_some() {
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
		let rest = TanzFaceSample::default();
		for _ in 0..7 {
			assert!(recognizer.sample(0.016, Vec2::Y, rest, &cfg).is_none());
		}
		let first = recognizer.sample(0.016, Vec2::Y, rest.with_a_press(), &cfg);
		assert_eq!(first.map(|gesture| gesture.button), Some(TanzButton::APress));
		assert!(first.is_some_and(|gesture| gesture.direction.y > 0.9));
		let held_a = TanzFaceSample::new(
			FaceButtonSample::new(true, false, false, 0.016),
			FaceButtonSample::default(),
		);
		assert!(recognizer.sample(0.016, Vec2::Y, held_a, &cfg).is_none());
		Ok(())
	}

	#[test]
	fn mode_toggle_fires_once_until_release() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let mut toggle = ModeToggle::default();
		for _ in 0..59 {
			assert!(!toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		}
		assert!(toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		assert!(!toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		assert!(!toggle.sample(dt, false, TANZ_TOGGLE_SECS, ControlMode::Standard));
		toggle.reset_for_mode(ControlMode::Tanzer);
		for _ in 0..59 {
			assert!(!toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Tanzer));
		}
		assert!(toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Tanzer));
		Ok(())
	}

	#[test]
	fn mode_charge_reaches_tanzer_in_sixty_steps_at_sixty_hz() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let mut toggle = ModeToggle::default();
		let mut toggled = false;
		for step in 0..60 {
			if toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Standard) {
				toggled = true;
				assert_eq!(step, 59);
			}
		}
		assert!(toggled);
		assert_eq!(toggle.bar(ControlMode::Standard), 1.0);
		Ok(())
	}

	#[test]
	fn mode_charge_returns_to_laufer_from_tanzer() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let mut toggle = ModeToggle::default();
		toggle.reset_for_mode(ControlMode::Tanzer);
		let mut toggled = false;
		for step in 0..60 {
			if toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Tanzer) {
				toggled = true;
				assert_eq!(step, 59);
			}
		}
		assert!(toggled);
		assert_eq!(toggle.bar(ControlMode::Tanzer), 0.0);
		Ok(())
	}

	#[test]
	fn early_release_drains_charge_back_to_laufer_rest() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let mut toggle = ModeToggle::default();
		for _ in 0..30 {
			assert!(!toggle.sample(dt, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		}
		let bar = toggle.bar(ControlMode::Standard);
		assert!(bar > 0.4 && bar < 0.6);
		for _ in 0..60 {
			assert!(!toggle.sample(dt, false, TANZ_TOGGLE_SECS, ControlMode::Standard));
		}
		assert_eq!(toggle.bar(ControlMode::Standard), 0.0);
		Ok(())
	}

	#[test]
	fn mode_charge_is_frame_rate_independent() -> anyhow::Result<()> {
		let mut slow = ModeToggle::default();
		let mut fast = ModeToggle::default();
		let mut slow_flips = 0;
		let mut fast_flips = 0;
		let mut t = 0.0;
		while t < 1.05 {
			if slow.sample(1.0 / 30.0, true, TANZ_TOGGLE_SECS, ControlMode::Standard) {
				slow_flips += 1;
			}
			t += 1.0 / 30.0;
		}
		t = 0.0;
		while t < 1.05 {
			if fast.sample(1.0 / 120.0, true, TANZ_TOGGLE_SECS, ControlMode::Standard) {
				fast_flips += 1;
			}
			t += 1.0 / 120.0;
		}
		assert_eq!(slow_flips, 1);
		assert_eq!(fast_flips, 1);
		Ok(())
	}

	/// Legacy Standard=0 / Tanzer=1 bar. Kept only to compare against [`ModeToggle::bar`].
	fn mode_charge_rest(mode: ControlMode) -> f32 {
		match mode {
			ControlMode::Standard => 0.0,
			ControlMode::Tanzer => 1.0,
		}
	}

	fn mode_charge_hold_target(mode: ControlMode) -> f32 {
		match mode {
			ControlMode::Standard => 1.0,
			ControlMode::Tanzer => 0.0,
		}
	}

	fn mode_charge_step_toward(progress: f32, target: f32, rate: f32) -> f32 {
		if progress < target {
			(progress + rate).min(target)
		} else if progress > target {
			(progress - rate).max(target)
		} else {
			progress
		}
	}

	fn charge_threshold_met(progress: f32, hold: f32, rest: f32) -> bool {
		const EPS: f32 = 1e-5;
		if hold > rest {
			progress + EPS >= hold
		} else {
			progress <= hold + EPS
		}
	}

	fn advance_mode_charge(
		progress: f32,
		fired: bool,
		dt: f32,
		chord: bool,
		charge_secs: f32,
		mode: ControlMode,
	) -> (f32, bool, bool) {
		let dt = dt.max(0.0);
		let rate = if charge_secs > 0.0 { dt / charge_secs } else { 1.0 };
		let rest = mode_charge_rest(mode);
		let hold = mode_charge_hold_target(mode);
		let mut fired = if chord { fired } else { false };
		if chord && fired {
			return (rest, true, false);
		}
		let target = if chord { hold } else { rest };
		let mut progress = mode_charge_step_toward(progress, target, rate);
		let toggled = if chord && charge_threshold_met(progress, hold, rest) {
			progress = hold;
			fired = true;
			true
		} else {
			false
		};
		(progress, fired, toggled)
	}

	#[derive(Clone, Copy)]
	struct ChargeFrame {
		dt: f32,
		chord: bool,
		charge_secs: f32,
	}

	fn hold_release(hold: usize, release: usize, dt: f32, charge_secs: f32) -> Vec<ChargeFrame> {
		let mut frames = Vec::with_capacity(hold + release);
		frames.extend(std::iter::repeat_n(ChargeFrame { dt, chord: true, charge_secs }, hold));
		frames.extend(std::iter::repeat_n(ChargeFrame { dt, chord: false, charge_secs }, release));
		frames
	}

	fn drive_old_bar(
		mode: ControlMode,
		start_bar: f32,
		start_fired: bool,
		frames: &[ChargeFrame],
	) -> Vec<(f32, bool, bool)> {
		let mut progress = start_bar;
		let mut fired = start_fired;
		let mut out = Vec::with_capacity(frames.len());
		for frame in frames {
			let (next, next_fired, toggled) = advance_mode_charge(
				progress,
				fired,
				frame.dt,
				frame.chord,
				frame.charge_secs,
				mode,
			);
			progress = next;
			fired = next_fired;
			out.push((next, next_fired, toggled));
		}
		out
	}

	fn drive_new_bar(
		mode: ControlMode,
		start_bar: f32,
		start_fired: bool,
		frames: &[ChargeFrame],
	) -> Vec<(f32, bool, bool)> {
		let charge = match mode {
			ControlMode::Standard => start_bar,
			ControlMode::Tanzer => 1.0 - start_bar,
		};
		let mut toggle = ModeToggle { charge, fired: start_fired, frozen: false };
		let mut out = Vec::with_capacity(frames.len());
		for frame in frames {
			let toggled = toggle.sample(frame.dt, frame.chord, frame.charge_secs, mode);
			out.push((toggle.bar(mode), toggle.fired, toggled));
		}
		out
	}

	fn assert_old_matches_new(
		mode: ControlMode,
		start_bar: f32,
		start_fired: bool,
		frames: &[ChargeFrame],
	) -> anyhow::Result<()> {
		let old = drive_old_bar(mode, start_bar, start_fired, frames);
		let new = drive_new_bar(mode, start_bar, start_fired, frames);
		if old.len() != new.len() {
			anyhow::bail!("old produced {} frames, new produced {}", old.len(), new.len());
		}
		for (i, (old_frame, new_frame)) in old.iter().zip(new.iter()).enumerate() {
			let (old_bar, old_fired, old_toggled) = *old_frame;
			let (new_bar, new_fired, new_toggled) = *new_frame;
			if (old_bar - new_bar).abs() > 1e-6 {
				anyhow::bail!("frame {i} {mode:?}: old bar {old_bar} != new bar {new_bar}");
			}
			if old_fired != new_fired || old_toggled != new_toggled {
				anyhow::bail!(
					"frame {i} {mode:?}: old latch ({old_fired}, {old_toggled}) != new ({new_fired}, {new_toggled})"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn old_and_new_bars_match_scripted_holds_in_both_modes() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let modes = [ControlMode::Standard, ControlMode::Tanzer];
		for mode in modes {
			let rest = mode_charge_rest(mode);
			// Full charge, latched extra hold, then release / rearm.
			assert_old_matches_new(mode, rest, false, &hold_release(65, 10, dt, TANZ_TOGGLE_SECS))?;
			// Partial hold then full decay back to rest.
			assert_old_matches_new(mode, rest, false, &hold_release(30, 60, dt, TANZ_TOGGLE_SECS))?;
			// Partial release, then resume through the threshold.
			let mut resume = hold_release(24, 12, dt, TANZ_TOGGLE_SECS);
			resume.extend(hold_release(50, 4, dt, TANZ_TOGGLE_SECS));
			assert_old_matches_new(mode, rest, false, &resume)?;
			// Zero-duration: completed bar and fire on the first hold frame.
			assert_old_matches_new(mode, rest, false, &hold_release(3, 2, dt, 0.0))?;
			// One oversized dt completes immediately without overshooting the bar.
			assert_old_matches_new(
				mode,
				rest,
				false,
				&[ChargeFrame { dt: 2.0, chord: true, charge_secs: TANZ_TOGGLE_SECS }],
			)?;
			// Zero dt must not advance or fire.
			assert_old_matches_new(
				mode,
				rest,
				false,
				&[ChargeFrame { dt: 0.0, chord: true, charge_secs: TANZ_TOGGLE_SECS }; 8],
			)?;
		}
		Ok(())
	}

	#[test]
	fn old_and_new_bars_match_after_apply_toggle_reset() -> anyhow::Result<()> {
		let dt = 1.0 / 60.0;
		let frames = hold_release(60, 0, dt, TANZ_TOGGLE_SECS);
		for start in [ControlMode::Standard, ControlMode::Tanzer] {
			let rest = mode_charge_rest(start);
			let old = drive_old_bar(start, rest, false, &frames);
			let new = drive_new_bar(start, rest, false, &frames);
			let last_old = old.last().ok_or_else(|| anyhow::anyhow!("old empty"))?;
			let last_new = new.last().ok_or_else(|| anyhow::anyhow!("new empty"))?;
			if !last_old.2 || !last_new.2 {
				anyhow::bail!("{start:?}: expected a toggle on the completing frame");
			}
			let flipped = start.toggle();
			let old_reset_bar = mode_charge_rest(flipped);
			let mut toggle = ModeToggle {
				charge: match start {
					ControlMode::Standard => last_new.0,
					ControlMode::Tanzer => 1.0 - last_new.0,
				},
				fired: last_new.1,
				frozen: false,
			};
			toggle.reset_for_mode(flipped);
			if (toggle.bar(flipped) - old_reset_bar).abs() > 1e-6 {
				anyhow::bail!(
					"{start:?}: reset bar {} != old rest {old_reset_bar}",
					toggle.bar(flipped)
				);
			}
			assert_old_matches_new(
				flipped,
				toggle.bar(flipped),
				toggle.fired,
				&hold_release(20, 4, dt, TANZ_TOGGLE_SECS),
			)?;
		}
		Ok(())
	}

	#[test]
	fn reassignment_cancels_partial_stick_history() {
		use maybraid_controllers_core::{InputSource, RoutingKind};

		let mut recognizer = TanzRecognizer::default();
		let cfg = TanzRecognizerConfig::default();
		recognizer.bind_source(InputSource::local_pad());
		for _ in 0..6 {
			assert!(recognizer
				.sample(1.0 / 60.0, Vec2::NEG_X, TanzFaceSample::default(), &cfg)
				.is_none());
		}
		assert!(recognizer.has_partial());
		let mut router = maybraid_controllers_core::InputRouter::default();
		router.assign_actor(
			InputSource::local_pad(),
			maybraid_controllers_core::ActorTarget::new(bevy::prelude::Entity::from_bits(1)),
		);
		router.take_events();
		router.assign_actor(
			InputSource::local_pad(),
			maybraid_controllers_core::ActorTarget::new(bevy::prelude::Entity::from_bits(2)),
		);
		let event = router.take_events().last().copied().expect("reassigned");
		assert_eq!(event.kind(), RoutingKind::Reassigned);
		assert_eq!(event.gesture(), GestureEffect::Cancel);
		recognizer.apply_routing(event);
		assert!(!recognizer.has_partial());
		assert!(recognizer.source().is_none());
	}

	#[test]
	fn disconnect_releases_a_bumper_hold() {
		let mut toggle = ModeToggle::default();
		for _ in 0..30 {
			assert!(!toggle.sample(1.0 / 60.0, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		}
		assert!(toggle.is_partial());
		toggle.apply_hold(HoldEffect::ReleaseNeutral);
		assert!(!toggle.is_partial());
		assert!(!toggle.sample(1.0 / 60.0, true, TANZ_TOGGLE_SECS, ControlMode::Standard));
		assert!(toggle.bar(ControlMode::Standard) < 0.05);
	}
}
