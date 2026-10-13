//! Elevation ops urbanization applies over a ground height.

/// Pads that rewrite a ground sample.
pub trait PadOps {
	fn modify_elevation(&self, height: f32, x: f32, z: f32) -> f32;
}
