//! Rig-agnostic gallop cycle parameters.
//!
//! Progress is owned by the controller; [`Gallop`] is a pure sampler at a normalized
//! cycle phase in `[0, 1)`. Quadruped rigs convert to [`QuadrupedGallop`](super::QuadrupedGallop)
//! before applying bound-style joint articulation.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gallop {
	/// Thigh forward/back stride amplitude (radians).
	pub stride: f32,
	/// Vertical lift during each bound; 1.0 = tuned quadruped default.
	pub bounce: f32,
	/// Spine compression during hind/front bounds; 1.0 = tuned default.
	pub bound_pitch: f32,
	/// Hind-to-front strike spacing; 1.0 = tuned quadruped default.
	pub phase_separation: f32,
}

impl Default for Gallop {
	fn default() -> Self {
		Self { stride: 1.1, bounce: 1.0, bound_pitch: 1.0, phase_separation: 1.0 }
	}
}
