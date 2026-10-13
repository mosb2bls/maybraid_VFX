//! Parameterized knobs + fit for [`super::PublicRestroom`].

use building_components::LabelStyle;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::PassageClearance;

use super::super::stall_layout::public_restroom::{
	PublicRestroomPacked, PublicRestroomRegions, RESTROOM_DOOR_HEADER_MIN,
	RESTROOM_DOOR_HEIGHT_MAX, RESTROOM_DOOR_HEIGHT_MIN, RESTROOM_DOOR_WIDTH_MAX,
	RESTROOM_DOOR_WIDTH_MIN, RESTROOM_SINK_DEPTH_MAX, RESTROOM_SINK_DEPTH_MIN, RESTROOM_SINK_MIN,
	RESTROOM_STALLS_MIN,
};

/// Noise / style knobs for [`super::PublicRestroom`].
#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomParameterized {
	pub style: LabelStyle,
	pub stalls_area_target: f32,
	pub stalls_seed_depth: f32,
	pub stalls_along_t: f32,
	pub sink_area_reserve: f32,
	pub sink_area_target: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub sink_depth: f32,
}

impl PublicRestroomParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		if PassageClearance::collect_faces(confines, host).is_empty() {
			return Err(FitError::TooSmall { reason: "restroom passage" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let usable = aabb2_area(host).max(1.0);
		let stalls_min = RESTROOM_STALLS_MIN * RESTROOM_STALLS_MIN;
		let sink_floor = RESTROOM_SINK_MIN * RESTROOM_SINK_MIN;

		// Stalls dominate, but reserve a kitchen-sized sink share so sinks can grow.
		let sink_area_reserve = cfg.sample_range_f32_4d(
			(usable * 0.12).max(sink_floor),
			(usable * 0.28).max(sink_floor + 0.5),
			c.x,
			c.y,
			c.z,
			91.0,
		);
		let lo = (usable * 0.50)
			.max(stalls_min)
			.min((usable - sink_area_reserve).max(stalls_min));
		let hi = (usable * 0.78).max(lo + 0.5);
		let stalls_area_target = cfg.sample_range_f32_4d(lo, hi, c.x, c.y, c.z, 90.0);
		// Primary sink grow target (within the door-side strip after packing).
		let sink_area_target = cfg.sample_range_f32_4d(
			sink_area_reserve * 0.55,
			sink_area_reserve.max(sink_floor + 0.5),
			c.x,
			c.y,
			c.z,
			99.0,
		);
		let stalls_seed_depth = cfg.sample_range_f32_4d(2.5, 4.5, c.x, c.y, c.z, 92.0);
		let stalls_along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 93.0);
		let door_width = cfg.sample_range_f32_4d(
			RESTROOM_DOOR_WIDTH_MIN,
			RESTROOM_DOOR_WIDTH_MAX,
			c.x,
			c.y,
			c.z,
			94.0,
		);
		let door_along_t = cfg.sample_range_f32_4d(0.15, 0.85, c.x, c.y, c.z, 95.0);
		let host_h = (confines.bounds.max.y - confines.bounds.min.y).max(1.0);
		let door_hi = RESTROOM_DOOR_HEIGHT_MAX
			.min((host_h - RESTROOM_DOOR_HEADER_MIN).max(RESTROOM_DOOR_HEIGHT_MIN));
		let door_height = cfg.sample_range_f32_4d(
			RESTROOM_DOOR_HEIGHT_MIN.min(door_hi),
			door_hi,
			c.x,
			c.y,
			c.z,
			96.0,
		);
		let sink_depth = cfg.sample_range_f32_4d(
			RESTROOM_SINK_DEPTH_MIN,
			RESTROOM_SINK_DEPTH_MAX,
			c.x,
			c.y,
			c.z,
			97.0,
		);
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 98.0));

		Ok(Self {
			style,
			stalls_area_target,
			stalls_seed_depth,
			stalls_along_t,
			sink_area_reserve,
			sink_area_target,
			door_width,
			door_along_t,
			door_height,
			sink_depth,
		})
	}

	fn regions(&self) -> PublicRestroomRegions {
		PublicRestroomRegions {
			stalls_area_target: self.stalls_area_target,
			stalls_seed_depth: self.stalls_seed_depth,
			stalls_along_t: self.stalls_along_t,
			sink_area_reserve: self.sink_area_reserve,
			sink_area_target: self.sink_area_target,
			door_width: self.door_width,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			sink_depth: self.sink_depth,
		}
	}
}

/// Geometry resolved from [`PublicRestroomParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomPlan {
	pub parameterized: PublicRestroomParameterized,
	pub packed: PublicRestroomPacked,
}

impl PublicRestroomPlan {
	pub fn from_parameterized(
		params: PublicRestroomParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.regions().pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
