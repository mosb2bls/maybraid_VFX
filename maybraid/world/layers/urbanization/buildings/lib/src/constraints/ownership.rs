//! Boundary ownership.

use bevy_math::bounding::Aabb2d;

use crate::constraints::region::{intersect_aabb2d, BoundaryRegionList, BoundaryRegionListExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoundaryOwnershipStatus {
	/// Owned by the cell (often implied by absence).
	#[default]
	Cell,
	/// Owned by a parent of the cell.
	Parent,
	/// Owned by a sibling of the cell.
	Sibling,
}

/// Ownership for one boundary face.
///
/// Whole and SubRegions are mutually exclusive at the type level.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryOwnershipEntry {
	Whole(BoundaryOwnershipStatus),
	/// [`Aabb2d`] keys are boundary-local (see [`BoundaryRegionList`]).
	SubRegions(BoundaryRegionList<BoundaryOwnershipStatus>),
}

impl Default for BoundaryOwnershipEntry {
	fn default() -> Self {
		Self::Whole(BoundaryOwnershipStatus::Cell)
	}
}

impl BoundaryOwnershipEntry {
	pub fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		match self {
			Self::Whole(status) => Self::Whole(*status),
			Self::SubRegions(regions) => Self::SubRegions(regions.clip_to_coverage(coverage)),
		}
	}

	pub fn owns_region_as_cell(&self, region: Aabb2d) -> bool {
		match self {
			Self::Whole(BoundaryOwnershipStatus::Cell) => true,
			Self::Whole(_) => false,
			Self::SubRegions(regions) => regions.iter().any(|(owned_region, status)| {
				*status == BoundaryOwnershipStatus::Cell
					&& intersect_aabb2d(*owned_region, region).is_some()
			}),
		}
	}
}
