//! Empty half-bathroom shell: passage keep-outs only.

use procedural_common::{aabb2_area, aabb3_to_plan, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::PassageClearance;

/// Minimum plan area (m²) for a half bathroom cell.
pub const MIN_AREA: f32 = 1.5 * 1.2;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResidentialHalfBathroomPacked {}

impl ResidentialHalfBathroomPacked {
	pub fn pack(confines: &Confines) -> Result<Self, FitError> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		if aabb2_area(host) + 1e-3 < MIN_AREA {
			return Err(FitError::TooSmall { reason: "residential half bathroom" });
		}
		if PassageClearance::collect_faces(confines, host).is_empty() {
			return Err(FitError::TooSmall { reason: "residential half bathroom passage" });
		}
		Ok(Self {})
	}
}
