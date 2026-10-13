//! Noise knobs + plan for [`super::CommonBedroom`].

use building_components::LabelStyle;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};

use super::layout::{CommonBedroomPacked, CommonBedroomRegions};

/// Opening-id scope for closet / ensuite doors authored by this usage area.
pub const SCOPE: &str = "common_bedroom";

/// Noise / fill knobs for [`super::CommonBedroom`].
///
/// Elevates the old bedroom fill budgets (`spaciousness` / `occupancy`) into the
/// parameterized → plan path so [`Self::sample`] picks them from spatial noise.
/// Ensuite / walk-in area targets follow the Bites seating↔kitchen split pattern:
/// grow the private room toward a noise target while reserving bedroom floor.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroomParameterized {
	pub style: LabelStyle,
	/// Multiplier on bed / nightstand / furniture / partition base footprints
	/// (`1.0` = nominal). Higher → each concept claims more floor.
	pub spaciousness: f32,
	/// Max floor-area fraction to allocate (leave about `1 - occupancy` empty).
	pub occupancy: f32,
	/// When true, prioritize placing beds flush against a host wall.
	pub bed_against_wall: bool,
	/// Plan-area target for the ensuite (grows in larger hosts).
	pub ensuite_area_target: f32,
	/// Plan area reserved for the bedroom program when growing the ensuite.
	pub bedroom_area_reserve: f32,
	/// Plan-area target for a walk-in closet (grows in larger hosts).
	pub walk_in_area_target: f32,
	pub closet_along_t: f32,
	pub walk_in_along_t: f32,
	pub ensuite_along_t: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

impl CommonBedroomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let usable = aabb2_area(aabb3_to_plan(&confines.bounds, PlanAxes::XZ)).max(1.0);

		// Bias toward roomier footprints than the old 1.0-centered default.
		let spaciousness =
			cfg.sample_range_f32_4d(1.05, 1.55, c.x, c.y, c.z, 10.0).clamp(0.75, 2.0);
		let occupancy = cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, 11.0).clamp(0.05, 1.0);
		let bed_against_wall = cfg.sample_unit_4d(c.x, c.y, c.z, 17.0) >= 0.4;

		// Ensuite: may claim a large share of the host (up to ~half) when space allows.
		let ensuite_min_area = 2.6 * 1.8;
		let ensuite_lo = (usable * 0.18).max(ensuite_min_area).min(usable.max(ensuite_min_area));
		let ensuite_hi = (usable * 0.50).max(ensuite_lo + 0.5);
		let ensuite_area_target =
			cfg.sample_range_f32_4d(ensuite_lo, ensuite_hi, c.x, c.y, c.z, 18.0);
		let bed_floor = 2.0 * 1.6 * spaciousness * spaciousness;
		// Leave at least ~half the host for the bedroom program when growing ensuite.
		let bedroom_area_reserve = cfg.sample_range_f32_4d(
			(usable * 0.35).max(bed_floor),
			(usable * 0.55).max(bed_floor + 1.0),
			c.x,
			c.y,
			c.z,
			19.0,
		);

		// Walk-in: larger than a shallow closet; grows modestly with host area.
		let walk_in_min_area = 2.4 * 1.5;
		let walk_lo = (usable * 0.06).max(walk_in_min_area).min(usable.max(walk_in_min_area));
		let walk_hi = (usable * 0.14).max(walk_lo + 0.5);
		let walk_in_area_target = cfg.sample_range_f32_4d(walk_lo, walk_hi, c.x, c.y, c.z, 20.0);

		let closet_along_t = cfg.sample_range_f32_4d(0.1, 0.9, c.x, c.y, c.z, 12.0);
		let walk_in_along_t = cfg.sample_range_f32_4d(0.1, 0.9, c.x, c.y, c.z, 21.0);
		let ensuite_along_t = cfg.sample_range_f32_4d(0.1, 0.9, c.x, c.y, c.z, 13.0);
		let scale = crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
		let door_width = cfg.sample_range_f32_4d(0.7 * scale, 1.05 * scale, c.x, c.y, c.z, 14.0);
		let door_along_t = cfg.sample_range_f32_4d(0.2, 0.8, c.x, c.y, c.z, 15.0);
		let host_h = (confines.bounds.max.y - confines.bounds.min.y).max(1.0);
		let door_hi = (2.2 * scale).min((host_h - 0.25).max(1.8 * scale));
		let door_height =
			cfg.sample_range_f32_4d((1.9 * scale).min(door_hi), door_hi, c.x, c.y, c.z, 16.0);

		Ok(Self {
			style: LabelStyle::Blue,
			spaciousness,
			occupancy,
			bed_against_wall,
			ensuite_area_target,
			bedroom_area_reserve,
			walk_in_area_target,
			closet_along_t,
			walk_in_along_t,
			ensuite_along_t,
			door_width,
			door_along_t,
			door_height,
		})
	}

	/// Explicit fill budgets (playground / tests); bed wall preference off by default.
	///
	/// Ensuite / walk-in targets default to modest multiples of their mins; pass a
	/// real host through [`Self::sample`] when area scaling with room size matters.
	pub fn with_fill(spaciousness: f32, occupancy: f32) -> Self {
		let spaciousness = spaciousness.max(1e-3);
		let ensuite_min = 2.6 * 1.8 * spaciousness.max(1.0);
		let walk_in_min = 2.4 * 1.5 * spaciousness.max(1.0);
		Self {
			style: LabelStyle::Blue,
			spaciousness,
			occupancy: occupancy.clamp(0.05, 1.0),
			bed_against_wall: false,
			// Pack clamps to host; bias high so large playground cells grow ensuite.
			ensuite_area_target: (ensuite_min * 2.5).max(24.0),
			bedroom_area_reserve: 12.0,
			walk_in_area_target: walk_in_min * 1.2,
			closet_along_t: 0.5,
			walk_in_along_t: 0.5,
			ensuite_along_t: 0.5,
			door_width: 0.85 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE,
			door_along_t: 0.5,
			door_height: crate::usage_areas::plan_access::DOOR_HEIGHT,
		}
	}
}

/// Sampled knobs + packed geometry for [`super::CommonBedroom`].
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroomPlan {
	pub parameterized: CommonBedroomParameterized,
	pub packed: CommonBedroomPacked,
}

impl CommonBedroomPlan {
	pub fn from_parameterized(
		params: CommonBedroomParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let regions = CommonBedroomRegions {
			spaciousness: params.spaciousness,
			occupancy: params.occupancy,
			bed_against_wall: params.bed_against_wall,
			ensuite_area_target: params.ensuite_area_target,
			bedroom_area_reserve: params.bedroom_area_reserve,
			walk_in_area_target: params.walk_in_area_target,
			closet_along_t: params.closet_along_t,
			walk_in_along_t: params.walk_in_along_t,
			ensuite_along_t: params.ensuite_along_t,
			door_width: params.door_width,
			door_along_t: params.door_along_t,
			door_height: params.door_height,
		};
		let packed = regions.pack(confines, noise)?;
		Ok(Self { parameterized: params, packed })
	}
}
