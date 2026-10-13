//! Local height field on a pad footprint.

/// How a pad sets height inside its footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadElevation {
	/// Replace incoming elevation with a constant height (building terrace).
	Flatten { height: f32 },
	/// Lerp from `height_a` to `height_b` along a reach (connecting path).
	Grade { height_a: f32, height_b: f32 },
}

impl PadElevation {
	pub fn representative_height(self) -> f32 {
		match self {
			Self::Flatten { height } => height,
			Self::Grade { height_a, height_b } => 0.5 * (height_a + height_b),
		}
	}
}
