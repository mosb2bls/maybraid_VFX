//! Parameterized knobs + plan for [`super::Kitchen`].

use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};

use super::layout::{KitchenCounterLayout, KitchenPacked, KitchenRegions};

pub const SCOPE: &str = "kitchen";

#[derive(Debug, Clone, PartialEq)]
pub struct KitchenParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
	/// When set, forces a counter subtype; otherwise noise picks galley / L / peninsula.
	pub layout: Option<KitchenCounterLayout>,
}

impl KitchenParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let fp = confines.footprint();
		let area = (fp.x * fp.y).max(1e-3);
		// Compact galleys stay tight; large rooms open up counters / islands.
		let size_t = ((area - 5.0) / 35.0).clamp(0.0, 1.0);
		let space_lo = 0.85 + 0.15 * size_t;
		let space_hi = 1.3 + 0.55 * size_t;
		let spaciousness = cfg
			.sample_range_f32_4d(space_lo, space_hi, c.x, c.y, c.z, 30.0)
			.clamp(0.75, 1.95);
		let occupancy = cfg
			.sample_range_f32_4d(0.28, 0.52 + 0.1 * size_t, c.x, c.y, c.z, 31.0)
			.clamp(0.1, 0.8);
		Ok(Self { style: LabelStyle::Yellow, spaciousness, occupancy, layout: None })
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Yellow,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
			layout: None,
		}
	}

	pub fn with_layout(mut self, layout: KitchenCounterLayout) -> Self {
		self.layout = Some(layout);
		self
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct KitchenPlan {
	pub parameterized: KitchenParameterized,
	pub packed: KitchenPacked,
}

impl KitchenPlan {
	pub fn from_parameterized(
		params: KitchenParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions = KitchenRegions {
			spaciousness: params.spaciousness,
			occupancy: params.occupancy,
			layout: params.layout,
		};
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
