//! Watershed-owned hydraulic support geometry.

pub mod ellipse;
pub mod reach_segment;

pub use ellipse::Ellipse;
pub use reach_segment::ReachSegment;

use bevy_math::Vec2;

/// Watershed-owned support geometry.
#[derive(Debug, Clone)]
pub enum HydroFootprint {
	/// Capsule / stadium for one reach segment.
	Reach(ReachSegment),
	/// Rotated elliptical disc (lake body).
	Ellipse(Ellipse),
}

impl HydroFootprint {
	pub fn sdf(&self, p: Vec2) -> f32 {
		match self {
			Self::Reach(seg) => seg.sdf(p),
			Self::Ellipse(e) => e.sdf(p),
		}
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		match self {
			Self::Reach(seg) => seg.aabb(),
			Self::Ellipse(e) => e.aabb(),
		}
	}
}
