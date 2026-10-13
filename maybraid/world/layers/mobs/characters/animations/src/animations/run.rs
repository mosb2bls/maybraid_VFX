//! Rig-agnostic run cycle parameters.
//!
//! Progress is owned by the controller; [`Run`] is a pure sampler at a normalized
//! cycle phase in `[0, 1)`. Humanoid rigs convert to [`UprightRun`](super::UprightRun)
//! before applying joint articulation.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Run {
	/// Femur forward/back stride amplitude (radians).
	pub stride: f32,
	/// Vertical bob; 1.0 = tuned upright default.
	pub bounce: f32,
	/// Hip/pelvis rotation in the gait; 1.0 = tuned upright default.
	pub rotation: f32,
}

impl Default for Run {
	fn default() -> Self {
		Self { stride: 1.05, bounce: 1.0, rotation: 1.0 }
	}
}
