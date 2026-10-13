//! Graded channel free-surface + transverse bowl along a reach.

/// Local \(Z\) along travel, local \(X\) across channel.
#[derive(Debug, Clone)]
pub struct ReachProfile {
	pub surface_a: f32,
	pub surface_b: f32,
	/// Centerline depth below \(W\); transverse bowl \(D_0 P(|X|)\).
	pub center_depth: f32,
}
