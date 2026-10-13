//! Parameterized knobs + plan for [`super::EatingArea`].

use bevy_math::bounding::Aabb2d;
use building_components::LabelStyle;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};
use crate::usage_areas::livable_quarters::dining_room::DiningRoomPlan;
use crate::usage_areas::livable_quarters::kitchen::{KitchenCounterLayout, KitchenPlan};

pub const SCOPE: &str = "eating_area";

/// Combined kitchen + optional dining layout.
#[derive(Debug, Clone, PartialEq)]
pub struct EatingAreaParameterized {
	pub style: LabelStyle,
	pub spaciousness: f32,
	pub occupancy: f32,
	/// Fraction of footprint claimed by the kitchen when splitting (0–1).
	pub kitchen_frac: f32,
	pub kitchen_layout: Option<KitchenCounterLayout>,
}

impl EatingAreaParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let fp = confines.footprint();
		let area = (fp.x * fp.y).max(1e-3);
		// Larger hosts bias toward roomier counters / a bigger kitchen share.
		let size_t = ((area - 8.0) / 50.0).clamp(0.0, 1.0);
		let space_lo = 0.85 + 0.2 * size_t;
		let space_hi = 1.35 + 0.45 * size_t;
		let frac_lo = 0.34 + 0.08 * size_t;
		let frac_hi = 0.52 + 0.16 * size_t;
		Ok(Self {
			style: LabelStyle::Yellow,
			spaciousness: cfg
				.sample_range_f32_4d(space_lo, space_hi, c.x, c.y, c.z, 40.0)
				.clamp(0.75, 1.9),
			occupancy: cfg.sample_range_f32_4d(0.28, 0.52, c.x, c.y, c.z, 41.0).clamp(0.1, 0.75),
			kitchen_frac: cfg
				.sample_range_f32_4d(frac_lo, frac_hi, c.x, c.y, c.z, 42.0)
				.clamp(0.3, 0.72),
			kitchen_layout: None,
		})
	}

	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		Self {
			style: LabelStyle::Yellow,
			spaciousness: spaciousness.max(1e-3),
			occupancy: occupancy.clamp(0.05, 1.0),
			kitchen_frac: 0.45,
			kitchen_layout: None,
		}
	}
}

/// Chosen layout after a successful pack.
#[derive(Debug, Clone, PartialEq)]
pub enum EatingAreaPacked {
	KitchenDining {
		kitchen: KitchenPlan,
		dining: DiningRoomPlan,
		kitchen_xz: Aabb2d,
		dining_xz: Aabb2d,
	},
	KitchenOnly {
		kitchen: KitchenPlan,
	},
}

#[derive(Debug, Clone, PartialEq)]
pub struct EatingAreaPlan {
	pub parameterized: EatingAreaParameterized,
	pub packed: EatingAreaPacked,
}
