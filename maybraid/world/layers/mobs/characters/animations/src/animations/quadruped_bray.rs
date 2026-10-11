//! Epiphant Bray: anticipation, lateral head flick, recovery.
//!
//! Tuned against the imported quadruped V0 neck channels (`neck_turn` = parent Y,
//! `neck_tilt` = parent Z, `neck_nod` = parent X). Sonic gameplay should key off
//! [`Self::SONIC_EMIT_NORM`], not pose sampling.

use crate::animations::smoothstep;
use crate::Progress;

/// Normalized clip time when the head passes through the flick apex (sonic emission).
pub const SONIC_EMIT_NORM: f32 = 0.32;

const ANTICIPATION_END: f32 = 0.22;
const FLICK_END: f32 = 0.42;

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
		smoothstep(t / ANTICIPATION_END)
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

	pub fn neck_turn(self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let ant = Self::anticipation_weight(t) * -self.anticipation * 0.35;
		let flick = if t < FLICK_END {
			Self::flick_weight(t) * self.flick_turn
		} else {
			self.flick_turn * (1.0 - Self::recovery_weight(t))
		};
		ant + flick
	}

	pub fn neck_nod(self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		let ant = Self::anticipation_weight(t) * -self.anticipation;
		let flick = if t < FLICK_END {
			Self::flick_weight(t) * self.flick_nod
		} else {
			self.flick_nod * (1.0 - Self::recovery_weight(t))
		};
		ant + flick
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
}
