//! Unit right-triangle panel atom.

use scene_ref::MirrorAxis;

/// Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\)
/// (right angle at the origin; third corner at local \((0, 0, -1)\)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RightTriangle {
	pub mirror: Option<MirrorAxis>,
}

impl Default for RightTriangle {
	fn default() -> Self {
		Self { mirror: None }
	}
}

impl RightTriangle {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn mirrored(mirror: MirrorAxis) -> Self {
		Self { mirror: Some(mirror) }
	}
}
