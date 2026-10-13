//! Parameterized knobs + plan for [`super::DiningRoom`].

use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

use super::layout::{DiningRoomPacked, DiningRoomRegions};

pub const SCOPE: &str = "dining_room";

#[derive(Debug, Clone, PartialEq)]
pub struct DiningRoomParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
}

impl DiningRoomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		Ok(Self {
			style: LabelStyle::Green,
			spaciousness: cfg.sample_range_f32_4d(0.95, 1.4, c.x, c.y, c.z, 40.0).clamp(0.75, 1.75),
			occupancy: cfg.sample_range_f32_4d(0.22, 0.45, c.x, c.y, c.z, 41.0).clamp(0.1, 0.7),
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Green,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
		}
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiningRoomPlan {
	pub parameterized: DiningRoomParameterized,
	pub packed: DiningRoomPacked,
}

impl DiningRoomPlan {
	pub fn from_parameterized(
		params: DiningRoomParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions =
			DiningRoomRegions { spaciousness: params.spaciousness, occupancy: params.occupancy };
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
