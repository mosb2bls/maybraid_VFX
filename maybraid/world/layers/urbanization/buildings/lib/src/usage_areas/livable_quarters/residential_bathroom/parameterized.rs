//! Parameterized knobs + plan for [`super::ResidentialBathroom`].

use building_components::LabelStyle;

use crate::fit::{Confines, FitError};

use super::layout::ResidentialBathroomPacked;

/// Opening-id scope for future fixtures / doors in this usage area.
pub const SCOPE: &str = "residential_bathroom";

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialBathroomParameterized {
	pub style: LabelStyle,
}

impl ResidentialBathroomParameterized {
	pub fn sample(
		_confines: &Confines,
		_noise: procedural_common::NoiseParams,
	) -> Result<Self, FitError> {
		Ok(Self { style: LabelStyle::Cyan })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialBathroomPlan {
	pub parameterized: ResidentialBathroomParameterized,
	pub packed: ResidentialBathroomPacked,
}

impl ResidentialBathroomPlan {
	pub fn from_parameterized(
		params: ResidentialBathroomParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = ResidentialBathroomPacked::pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
