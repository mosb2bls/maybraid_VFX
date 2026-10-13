//! Parameterized knobs + plan for [`super::ResidentialHalfBathroom`].

use building_components::LabelStyle;

use crate::fit::{Confines, FitError};

use super::layout::ResidentialHalfBathroomPacked;

pub const SCOPE: &str = "residential_half_bathroom";

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialHalfBathroomParameterized {
	pub style: LabelStyle,
}

impl ResidentialHalfBathroomParameterized {
	pub fn sample(
		_confines: &Confines,
		_noise: procedural_common::NoiseParams,
	) -> Result<Self, FitError> {
		Ok(Self { style: LabelStyle::Cyan })
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialHalfBathroomPlan {
	pub parameterized: ResidentialHalfBathroomParameterized,
	pub packed: ResidentialHalfBathroomPacked,
}

impl ResidentialHalfBathroomPlan {
	pub fn from_parameterized(
		params: ResidentialHalfBathroomParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = ResidentialHalfBathroomPacked::pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
