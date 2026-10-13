//! Flat free surface with a radial bowl under an elliptical footprint.

/// Flat \(W\); bowl in ellipse-normalized \(u\).
#[derive(Debug, Clone)]
pub struct RadialBowl {
	pub surface: f32,
	pub center_depth: f32,
}
