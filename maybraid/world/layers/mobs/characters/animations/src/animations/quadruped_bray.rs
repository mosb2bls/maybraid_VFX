//! Epiphant Bray: anticipation, lateral head flick, recovery.
//!
//! Tuned against the imported quadruped V0 neck channels (`neck_turn` = parent Y,
//! `neck_tilt` = parent Z, `neck_nod` = parent X). Sonic gameplay should key off
//! [`Self::SONIC_EMIT_NORM`], not pose sampling.

use bevy::prelude::Vec3;

use crate::animations::smoothstep;
use crate::Progress;

/// Normalized clip time when the head passes through the flick apex (sonic emission).
pub const SONIC_EMIT_NORM: f32 = 0.32;

/// Epiphant head rig bone that carries the nose mesh (see `species/epiphant/recipe.rs`).
pub const MOUTH_EMIT_SOCKET: &str = "nose_socket";

/// Fallback mouth origin in **body** `neck` local space when `nose_socket` is unavailable (+Z fight-forward).
pub const MOUTH_EMIT_NECK_LOCAL: Vec3 = Vec3::new(0.0, 0.05, 0.55);

const ANTICIPATION_END: f32 = 0.22;
const FLICK_END: f32 = 0.42;
/// Tail fade so anticipation eases out into the flick window (no step at [`ANTICIPATION_END`]).
const ANTICIPATION_CROSSFADE: f32 = 0.04;
const FLICK_END_SAMPLE_EPS: f32 = 1e-5;

/// Bray intensity knobs. Defaults match Bikanwarda's ordinary-scale preview.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpiphantBray {
	/// Lateral head sweep at the flick (radians, `neck_turn`).
	pub flick_turn: f32,
	/// Upward nod at the flick (`neck_nod`).
	pub flick_nod: f32,
	/// Anticipation pull-back before the snap.
	pub anticipation: f32,
}

impl Default for EpiphantBray {
	fn default() -> Self {
		Self { flick_turn: 0.55, flick_nod: 0.28, anticipation: 0.18 }
	}
}

impl EpiphantBray {
	pub fn anticipation_weight(progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t >= ANTICIPATION_END {
			return 0.0;
		}
		let rise = smoothstep(t / ANTICIPATION_END);
		let tail = if t <= ANTICIPATION_END - ANTICIPATION_CROSSFADE {
			1.0
		} else {
			let u = (ANTICIPATION_END - t) / ANTICIPATION_CROSSFADE;
			smoothstep(u)
		};
		rise * tail
	}

	pub fn flick_weight(progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < ANTICIPATION_END || t >= FLICK_END {
			return 0.0;
		}
		let u = (t - ANTICIPATION_END) / (FLICK_END - ANTICIPATION_END);
		// Peak near the middle of the flick window.
		(1.0 - (2.0 * u - 1.0).abs()).max(0.0)
	}

	pub fn recovery_weight(progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < FLICK_END {
			return 0.0;
		}
		smoothstep((t - FLICK_END) / (1.0 - FLICK_END).max(f32::EPSILON))
	}

	/// Flick envelope for neck channels: triangular flick, then optional decay from the
	/// value at [`FLICK_END`] (not a fresh snap to full amplitude).
	pub fn flick_neck_scale(progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < FLICK_END {
			return Self::flick_weight(t);
		}
		let at_flick_end =
			Self::flick_weight((FLICK_END - FLICK_END_SAMPLE_EPS).max(ANTICIPATION_END));
		at_flick_end * (1.0 - Self::recovery_weight(t))
	}

	pub fn neck_turn(self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let ant = Self::anticipation_weight(t) * -self.anticipation * 0.35;
		ant + Self::flick_neck_scale(t) * self.flick_turn
	}

	pub fn neck_nod(self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let ant = Self::anticipation_weight(t) * -self.anticipation;
		ant + Self::flick_neck_scale(t) * self.flick_nod
	}

	pub fn spine_gather(self, progress: f32) -> f32 {
		let flick = Self::flick_weight(progress);
		self.anticipation * 0.25 * Self::anticipation_weight(progress) + flick * 0.12
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sonic_emit_lies_on_the_flick_peak() {
		let at_emit = EpiphantBray::flick_weight(SONIC_EMIT_NORM);
		let before = EpiphantBray::flick_weight(SONIC_EMIT_NORM - 0.08);
		let after = EpiphantBray::flick_weight(SONIC_EMIT_NORM + 0.08);
		assert!(at_emit > before);
		assert!(at_emit > after);
		assert!(at_emit > 0.85);
	}

	#[test]
	fn recovery_returns_neck_channels_toward_rest() {
		let bray = EpiphantBray::default();
		let peak_turn = bray.neck_turn(0.34);
		let end_turn = bray.neck_turn(1.0);
		assert!(peak_turn.abs() > 0.15);
		assert!(end_turn.abs() < 0.05);
	}

	/// Largest per-step change on a uniform progress grid (catches visible pose pops).
	fn max_step_delta(sample: impl Fn(f32) -> f32, steps: usize) -> f32 {
		let mut max = 0.0_f32;
		for i in 0..steps {
			let t0 = i as f32 / steps as f32;
			let t1 = (i + 1) as f32 / steps as f32;
			max = max.max((sample(t1) - sample(t0)).abs());
		}
		max
	}

	fn assert_continuous_at(sample: impl Fn(f32) -> f32, boundary: f32, eps: f32, max_jump: f32) {
		let before = sample(boundary - eps);
		let after = sample(boundary + eps);
		assert!(
			(after - before).abs() <= max_jump,
			"discontinuity at {boundary}: {before} -> {after}"
		);
	}

	#[test]
	fn phase_boundaries_keep_neck_channels_continuous() {
		let bray = EpiphantBray::default();
		let turn = |t: f32| bray.neck_turn(t);
		let nod = |t: f32| bray.neck_nod(t);
		const EPS: f32 = 1e-4;
		const BOUND_JUMP: f32 = 0.02;
		for boundary in [ANTICIPATION_END, FLICK_END] {
			assert_continuous_at(turn, boundary, EPS, BOUND_JUMP);
			assert_continuous_at(nod, boundary, EPS, BOUND_JUMP);
		}
		// No single 1/240-step pop larger than a few degrees at default knobs.
		const GRID: usize = 240;
		const MAX_STEP: f32 = 0.06;
		assert!(max_step_delta(turn, GRID) <= MAX_STEP, "neck_turn step");
		assert!(max_step_delta(nod, GRID) <= MAX_STEP, "neck_nod step");
	}

	#[test]
	fn flick_end_does_not_reapply_full_turn_amplitude() {
		let bray = EpiphantBray::default();
		let just_before = bray.neck_turn(FLICK_END - 1e-3);
		let just_after = bray.neck_turn(FLICK_END + 1e-3);
		assert!(
			just_after.abs() <= just_before.abs() + 0.02,
			"recovery must continue from flick end, not snap: {just_before} -> {just_after}"
		);
	}
}
