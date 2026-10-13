//! Parameterized knobs + plan for [`super::LivingRoom`].

use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

use super::layout::{LivingRoomPacked, LivingRoomRegions};

pub const SCOPE: &str = "living_room";

#[derive(Debug, Clone, PartialEq)]
pub struct LivingRoomParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl LivingRoomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			style: LabelStyle::Orange,
			spaciousness: cfg.sample_range_f32_4d(1.0, 1.5, c.x, c.y, c.z, 50.0).clamp(0.8, 1.85),
			occupancy: cfg.sample_range_f32_4d(0.25, 0.5, c.x, c.y, c.z, 51.0).clamp(0.12, 0.75),
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Orange,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivingRoomPlan {
	pub parameterized: LivingRoomParameterized,
	pub packed: LivingRoomPacked,
}

impl LivingRoomPlan {
	pub fn from_parameterized(
		params: LivingRoomParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions =
			LivingRoomRegions { spaciousness: params.spaciousness, occupancy: params.occupancy };
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
