//! Parameterized knobs + fit for [`super::PartsStall`].

use building_components::LabelStyle;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};

use super::super::stall_layout::parts::{
	PartsPacked, PartsRegions, PARTS_DOOR_HEADER_MIN, PARTS_DOOR_HEIGHT_MAX, PARTS_DOOR_HEIGHT_MIN,
	PARTS_DOOR_WIDTH_MAX, PARTS_DOOR_WIDTH_MIN, PARTS_OFFICE_MIN, PARTS_REGION_MIN,
};
use crate::usage_areas::clearance::PassageClearance;

/// Noise / style knobs for [`super::PartsStall`].
#[derive(Debug, Clone, PartialEq)]
pub struct PartsStallParameterized {
	pub style: LabelStyle,
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	pub parts_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

impl PartsStallParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		if PassageClearance::collect_faces(confines, host).is_empty() {
			return Err(FitError::TooSmall { reason: "parts passage" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let usable = aabb2_area(host).max(1.0);
		let office_min = PARTS_OFFICE_MIN * PARTS_OFFICE_MIN;
		let lo = (usable * 0.12).max(office_min).min(usable.max(office_min));
		let hi = (usable * 0.30).max(lo + 0.5);
		let office_area_target = cfg.sample_range_f32_4d(lo, hi, c.x, c.y, c.z, 70.0);
		let parts_floor = PARTS_REGION_MIN * PARTS_REGION_MIN;
		let parts_area_reserve = cfg.sample_range_f32_4d(
			parts_floor,
			(usable * 0.55).max(parts_floor + 1.0),
			c.x,
			c.y,
			c.z,
			71.0,
		);
		let office_seed_depth = cfg.sample_range_f32_4d(2.0, 3.0, c.x, c.y, c.z, 72.0);
		let office_along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 73.0);
		let door_width = cfg.sample_range_f32_4d(
			PARTS_DOOR_WIDTH_MIN,
			PARTS_DOOR_WIDTH_MAX,
			c.x,
			c.y,
			c.z,
			74.0,
		);
		let door_along_t = cfg.sample_range_f32_4d(0.15, 0.85, c.x, c.y, c.z, 75.0);
		let host_h = (confines.bounds.max.y - confines.bounds.min.y).max(1.0);
		let door_hi =
			PARTS_DOOR_HEIGHT_MAX.min((host_h - PARTS_DOOR_HEADER_MIN).max(PARTS_DOOR_HEIGHT_MIN));
		let door_height = cfg.sample_range_f32_4d(
			PARTS_DOOR_HEIGHT_MIN.min(door_hi),
			door_hi,
			c.x,
			c.y,
			c.z,
			76.0,
		);
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 77.0));

		Ok(Self {
			style,
			office_area_target,
			office_seed_depth,
			office_along_t,
			parts_area_reserve,
			door_width,
			door_along_t,
			door_height,
		})
	}

	fn regions(&self) -> PartsRegions {
		PartsRegions {
			office_area_target: self.office_area_target,
			office_seed_depth: self.office_seed_depth,
			office_along_t: self.office_along_t,
			parts_area_reserve: self.parts_area_reserve,
			door_width: self.door_width,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
		}
	}
}

/// Geometry resolved from [`PartsStallParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct PartsStallPlan {
	pub parameterized: PartsStallParameterized,
	pub packed: PartsPacked,
}

impl PartsStallPlan {
	pub fn from_parameterized(
		params: PartsStallParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.regions().pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
