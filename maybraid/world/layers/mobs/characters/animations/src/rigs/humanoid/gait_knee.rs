//! Shared swing-phase knee lift timing for biped gait clips.
//!
//! Walk and run use different stance/peak flexion values, but sharing the same
//! lift envelope keeps `Mix<Walk, Run>` knee timing aligned ([#981](https://github.com/ramate-io/maybraid/pull/981)).

/// Leg-cycle phase where swing knee lift begins (after early stance).
pub const SWING_KNEE_LIFT_START: f32 = 0.35;
/// Span of the lift envelope; peaks at `START + SPAN / 2`, returns to zero by `START + SPAN`.
pub const SWING_KNEE_LIFT_SPAN: f32 = 0.50;

/// Unitless lift envelope over one leg cycle. Zero during early stance, peaks mid-swing.
pub fn swing_knee_lift_envelope(leg_phase: f32) -> f32 {
	let p = leg_phase.fract();
	if p < SWING_KNEE_LIFT_START {
		return 0.0;
	}
	let u = ((p - SWING_KNEE_LIFT_START) / SWING_KNEE_LIFT_SPAN).clamp(0.0, 1.0);
	(u * std::f32::consts::PI).sin()
}

/// Interpolate knee flexion from `stance` toward `peak` using the shared swing envelope.
pub fn lerp_swing_knee(leg_phase: f32, stance: f32, peak: f32) -> f32 {
	stance + swing_knee_lift_envelope(leg_phase) * (peak - stance)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn envelope_is_zero_during_early_stance() {
		for p in [0.0, 0.1, 0.34] {
			assert!(swing_knee_lift_envelope(p).abs() < 1e-5, "phase {p}");
		}
	}

	#[test]
	fn envelope_peaks_mid_swing_before_contact() {
		let peak_phase = SWING_KNEE_LIFT_START + SWING_KNEE_LIFT_SPAN * 0.5;
		let before = swing_knee_lift_envelope(peak_phase - 0.05);
		let at = swing_knee_lift_envelope(peak_phase);
		let after = swing_knee_lift_envelope(peak_phase + 0.05);
		assert!(at > before && at > after, "peak at {peak_phase}: {before} {at} {after}");
		assert!(peak_phase < 0.75, "peak should precede late-swing contact");
	}
}
