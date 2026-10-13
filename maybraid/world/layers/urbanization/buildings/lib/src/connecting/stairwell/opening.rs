//! Horizontal shaft-face opening used only to allocate a [`super::WellAabb`].

use bevy_math::Vec3;

use crate::openings::MappedOpening;

/// Horizontal shaft-face opening, typed for [`super::ConnectingStairwell`].
///
/// The quad lies in plan. Lower edge = walk-on (or, on the upper face, the
/// walk-off contact). `orientation` is XZ into the well from that edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StairwellOpening(MappedOpening);

impl StairwellOpening {
	pub fn new(mapped: MappedOpening) -> Self {
		Self(mapped)
	}

	pub fn mapped(self) -> MappedOpening {
		self.0
	}

	pub fn corners(self) -> [Vec3; 4] {
		let (bl, br, tl, tr) = self.0.endpoint_corners();
		[bl, br, tl, tr]
	}

	/// Centroid of the horizontal shaft face.
	pub fn face_center(self) -> Vec3 {
		let [bl, br, tl, tr] = self.corners();
		(bl + br + tl + tr) * 0.25
	}

	/// Midpoint of the lower edge (walk-on on the floor face; walk-off contact above).
	pub fn walk_on_mid(self) -> Vec3 {
		let [bl, br, ..] = self.corners();
		(bl + br) * 0.5
	}
}

impl From<MappedOpening> for StairwellOpening {
	fn from(mapped: MappedOpening) -> Self {
		Self::new(mapped)
	}
}

impl From<StairwellOpening> for MappedOpening {
	fn from(opening: StairwellOpening) -> Self {
		opening.mapped()
	}
}
