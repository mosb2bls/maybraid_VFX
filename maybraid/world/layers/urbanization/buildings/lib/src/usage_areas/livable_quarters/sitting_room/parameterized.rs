//! Parameterized knobs + plan for [`super::SittingRoom`].

use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

use super::layout::{SittingRoomPacked, SittingRoomRegions};

pub const SCOPE: &str = "sitting_room";

#[derive(Debug, Clone, PartialEq)]
pub struct SittingRoomParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl SittingRoomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			style: LabelStyle::Orange,
			spaciousness: cfg.sample_range_f32_4d(0.85, 1.15, c.x, c.y, c.z, 60.0).clamp(0.7, 1.4),
			occupancy: cfg.sample_range_f32_4d(0.18, 0.38, c.x, c.y, c.z, 61.0).clamp(0.08, 0.55),
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Orange,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 0.6),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct SittingRoomPlan {
	pub parameterized: SittingRoomParameterized,
	pub packed: SittingRoomPacked,
}

impl SittingRoomPlan {
	pub fn from_parameterized(
		params: SittingRoomParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions =
			SittingRoomRegions { spaciousness: params.spaciousness, occupancy: params.occupancy };
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
