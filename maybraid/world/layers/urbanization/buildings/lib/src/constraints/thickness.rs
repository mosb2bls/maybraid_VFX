//! Boundary thickness.

use bevy_math::bounding::Aabb2d;

use crate::constraints::region::{BoundaryRegionList, BoundaryRegionListExt};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BoundaryThicknessEntry {
	/// Fallback thickness when the boundary is one thickness.
	pub fallback: f32,
	pub sub_regions: BoundaryRegionList<f32>,
}

impl BoundaryThicknessEntry {
	pub fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		Self { fallback: self.fallback, sub_regions: self.sub_regions.clip_to_coverage(coverage) }
	}
}
