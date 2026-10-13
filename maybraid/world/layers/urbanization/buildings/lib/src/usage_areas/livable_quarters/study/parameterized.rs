//! Parameterized knobs + plan for [`super::Study`].

use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

use super::layout::{StudyPacked, StudyRegions};

pub const SCOPE: &str = "study";

#[derive(Debug, Clone, PartialEq)]
pub struct StudyParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl StudyParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			style: LabelStyle::Blue,
			spaciousness: cfg.sample_range_f32_4d(0.9, 1.25, c.x, c.y, c.z, 70.0).clamp(0.75, 1.5),
			occupancy: cfg.sample_range_f32_4d(0.2, 0.42, c.x, c.y, c.z, 71.0).clamp(0.1, 0.65),
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Blue,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct StudyPlan {
	pub parameterized: StudyParameterized,
	pub packed: StudyPacked,
}

impl StudyPlan {
	pub fn from_parameterized(
		params: StudyParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions =
			StudyRegions { spaciousness: params.spaciousness, occupancy: params.occupancy };
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
