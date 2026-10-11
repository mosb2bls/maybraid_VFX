//! Quadruped idle: graze, look up, then a short shake.
//!
//! Progress is owned by the controller and may grow past `1.0`. The graze
//! sequence is a designed 1D envelope on a longer period; it starts and ends
//! at rest so a period wrap is quiet. A small unwrapped sway keeps rest from
//! looking frozen.
//!
//! Neck knobs follow [`crate::rigs::quadruped::apply::apply_neck_axes`]:
//! **flex** is neck tilt, **twist** is neck nod, and **swing** is neck turn.

use crate::animations::{burst_envelope, smoothstep, DEFAULT_BURST_EDGE};

const DEFAULT_GRAZE_NECK: f32 = 0.68;
const DEFAULT_GRAZE_PITCH: f32 = 0.45;
const DEFAULT_LOOK_NECK: f32 = 0.28;
const DEFAULT_LOOK_PITCH: f32 = 0.22;
const DEFAULT_SHAKE: f32 = 0.10;
const DEFAULT_LUMBAR: f32 = 0.15;
const DEFAULT_SWAY: f32 = 0.018;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedIdle {
	/// Side-to-side while the muzzle is down (neck tilt, radians).
	pub graze_neck: f32,
	/// Up / down while the muzzle is down (neck nod, radians).
	pub graze_pitch: f32,
	/// Side-to-side while looking up (neck tilt, radians).
	pub look_neck: f32,
	/// Up / down while looking up (neck nod, radians).
	pub look_pitch: f32,
	/// Roll during the shake burst (neck turn, radians).
	pub shake: f32,
	/// Lumbar gather while grazing (radians).
	pub lumbar: f32,
	/// Quiet weight-shift amplitude (radians).
	pub sway: f32,
}

impl Default for QuadrupedIdle {
	fn default() -> Self {
		Self {
			graze_neck: DEFAULT_GRAZE_NECK,
			graze_pitch: DEFAULT_GRAZE_PITCH,
			look_neck: DEFAULT_LOOK_NECK,
			look_pitch: DEFAULT_LOOK_PITCH,
			shake: DEFAULT_SHAKE,
			lumbar: DEFAULT_LUMBAR,
			sway: DEFAULT_SWAY,
		}
	}
}

impl QuadrupedIdle {
	/// Quiet time plus one graze / look / shake, in progress units (~13 s at idle speed).
	pub const PERIOD: f32 = 2.6;
	pub const REST_END: f32 = 0.18;
	pub const LOWER_END: f32 = 0.48;
	pub const GRAZE_END: f32 = 1.08;
	pub const RISE_END: f32 = 1.38;
	pub const LOOK_END: f32 = 1.62;
	pub const SHAKE_END: f32 = 1.80;
	pub const SETTLE_END: f32 = 2.12;

	pub const SWAY_FREQ: f32 = 0.7;
	pub const BOB_FREQ: f32 = 1.8;
	pub const SHAKE_FREQ: f32 = 11.0;
	pub const GLANCE_FREQ: f32 = 0.45;

	/// Mid-graze hold. Useful for tests and a readable still frame.
	pub fn graze_peak() -> f32 {
		(Self::LOWER_END + Self::GRAZE_END) * 0.5
	}

	/// Mid look-up hold.
	pub fn look_peak() -> f32 {
		(Self::RISE_END + Self::LOOK_END) * 0.5
	}

	/// Mid shake burst.
	pub fn shake_peak() -> f32 {
		(Self::LOOK_END + Self::SHAKE_END) * 0.5
	}

	/// `1` while the muzzle is down, `0` at rest. Closed at the period wrap.
	pub fn graze_weight(progress: f32) -> f32 {
		let t = progress.rem_euclid(Self::PERIOD);
		if t < Self::REST_END || t >= Self::RISE_END {
			0.0
		} else if t < Self::LOWER_END {
			smoothstep((t - Self::REST_END) / (Self::LOWER_END - Self::REST_END))
		} else if t < Self::GRAZE_END {
			1.0
		} else {
			1.0 - smoothstep((t - Self::GRAZE_END) / (Self::RISE_END - Self::GRAZE_END))
		}
	}

	/// `1` while the head is raised, `0` at rest. Closed at the period wrap.
	pub fn look_weight(progress: f32) -> f32 {
		let t = progress.rem_euclid(Self::PERIOD);
		if t < Self::GRAZE_END || t >= Self::SETTLE_END {
			0.0
		} else if t < Self::RISE_END {
			smoothstep((t - Self::GRAZE_END) / (Self::RISE_END - Self::GRAZE_END))
		} else if t < Self::LOOK_END {
			1.0
		} else {
			1.0 - smoothstep((t - Self::LOOK_END) / (Self::SETTLE_END - Self::LOOK_END))
		}
	}

	/// `0` outside the burst, `1` at the hold. Continuous across the period wrap.
	pub fn shake_weight(progress: f32) -> f32 {
		let t = progress.rem_euclid(Self::PERIOD);
		if t < Self::LOOK_END || t >= Self::SHAKE_END {
			return 0.0;
		}
		burst_envelope(
			(t - Self::LOOK_END) / (Self::SHAKE_END - Self::LOOK_END),
			DEFAULT_BURST_EDGE,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_graze_is_deeper_than_the_look() {
		let idle = QuadrupedIdle::default();
		assert!(idle.graze_neck > idle.look_neck);
		assert!(idle.graze_pitch > idle.look_pitch);
		assert!(idle.look_neck > idle.shake);
		assert!(idle.sway < idle.shake);
	}

	#[test]
	fn graze_is_quiet_outside_the_bend() {
		assert!(QuadrupedIdle::graze_weight(0.0) < 1e-5);
		assert!(QuadrupedIdle::graze_weight(QuadrupedIdle::REST_END * 0.5) < 1e-5);
		assert!(QuadrupedIdle::graze_weight(QuadrupedIdle::graze_peak()) > 0.9);
		assert!(QuadrupedIdle::graze_weight(QuadrupedIdle::look_peak()) < 1e-5);
	}

	#[test]
	fn look_follows_the_rise() {
		assert!(QuadrupedIdle::look_weight(QuadrupedIdle::graze_peak()) < 1e-5);
		assert!(QuadrupedIdle::look_weight(QuadrupedIdle::look_peak()) > 0.9);
		assert!(QuadrupedIdle::look_weight(QuadrupedIdle::SETTLE_END) < 1e-5);
	}

	#[test]
	fn shake_is_a_closed_burst_after_the_look() {
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::look_peak()) < 1e-5);
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::shake_peak()) > 0.9);
		assert!(QuadrupedIdle::shake_weight(0.0) < 1e-5);
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::PERIOD) < 1e-5);
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::PERIOD - 0.01) < 1e-5);
	}

	#[test]
	fn envelopes_are_closed_at_the_period_wrap() {
		assert!(QuadrupedIdle::graze_weight(0.0) < 1e-5);
		assert!(QuadrupedIdle::graze_weight(QuadrupedIdle::PERIOD) < 1e-5);
		assert!(QuadrupedIdle::look_weight(0.0) < 1e-5);
		assert!(QuadrupedIdle::look_weight(QuadrupedIdle::PERIOD) < 1e-5);
	}
}
