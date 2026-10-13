//! Parameterized knobs + fit for [`super::MiniMart`].

use building_components::LabelStyle;
use procedural_common::{
	aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, OptionalFaceBand, PlanAxes,
};

use crate::fit::{Confines, FitError};

use crate::usage_areas::clearance::{PassageClearance, PlanHost};

use super::super::stall_layout::mini_mart::{
	MINI_MART_AISLES_MIN, MINI_MART_DOOR_HEADER_MIN, MINI_MART_DOOR_HEIGHT_MAX,
	MINI_MART_DOOR_HEIGHT_MIN, MINI_MART_DOOR_WIDTH_MAX, MINI_MART_DOOR_WIDTH_MIN,
	MINI_MART_OFFICE_LONG_MIN, MINI_MART_OFFICE_SHORT_MIN, MINI_MART_REGISTER_MIN,
	MINI_MART_SHELF_DEPTH_MAX, MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_PLACE_RATE,
};
use super::super::stall_layout::{MiniMartPacked, MiniMartRegions, MiniMartShelfSpec};

/// Noise / style knobs for [`super::MiniMart`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartParameterized {
	pub style: LabelStyle,
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	pub aisles_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub register_along_t: f32,
	pub register_seed_depth: f32,
	pub shelves: Vec<MiniMartShelfSpec>,
}

impl MiniMartParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall { reason: "mini mart passage" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let usable = aabb2_area(host).max(1.0);
		let office_min_area = MINI_MART_OFFICE_LONG_MIN * MINI_MART_OFFICE_SHORT_MIN;
		let lo = (usable * 0.12).max(office_min_area).min(usable.max(office_min_area));
		let hi = (usable * 0.28).max(lo + 0.5);
		let office_area_target = cfg.sample_range_f32_4d(lo, hi, c.x, c.y, c.z, 50.0);
		let sales_floor = MINI_MART_AISLES_MIN * MINI_MART_AISLES_MIN
			+ MINI_MART_REGISTER_MIN * MINI_MART_REGISTER_MIN;
		let aisles_area_reserve = cfg.sample_range_f32_4d(
			sales_floor,
			(usable * 0.55).max(sales_floor + 1.0),
			c.x,
			c.y,
			c.z,
			51.0,
		);
		let office_seed_depth = cfg.sample_range_f32_4d(2.0, 3.2, c.x, c.y, c.z, 52.0);
		let office_along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 53.0);
		let door_width = cfg.sample_range_f32_4d(
			MINI_MART_DOOR_WIDTH_MIN,
			MINI_MART_DOOR_WIDTH_MAX,
			c.x,
			c.y,
			c.z,
			54.0,
		);
		let door_along_t = cfg.sample_range_f32_4d(0.15, 0.85, c.x, c.y, c.z, 55.0);
		let host_h = (confines.bounds.max.y - confines.bounds.min.y).max(1.0);
		let door_hi = MINI_MART_DOOR_HEIGHT_MAX
			.min((host_h - MINI_MART_DOOR_HEADER_MIN).max(MINI_MART_DOOR_HEIGHT_MIN));
		let door_height = cfg.sample_range_f32_4d(
			MINI_MART_DOOR_HEIGHT_MIN.min(door_hi),
			door_hi,
			c.x,
			c.y,
			c.z,
			59.0,
		);
		let register_along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 56.0);
		let register_seed_depth = cfg.sample_range_f32_4d(2.0, 2.8, c.x, c.y, c.z, 57.0);
		let style = LabelStyle::from_unit(cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, 58.0));

		let free_faces = PlanHost::free_faces(host, &passage_faces);
		let shelves = free_faces
			.into_iter()
			.enumerate()
			.map(|(i, face)| {
				let salt = 60.0 + i as f32 * 11.0;
				let place_u = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, salt);
				let along_max = face.along_len().max(0.5);
				let along = cfg.sample_range_f32_4d(
					(along_max * 0.35).max(0.5),
					along_max,
					c.x,
					c.y,
					c.z,
					salt + 1.0,
				);
				let depth = cfg.sample_range_f32_4d(
					MINI_MART_SHELF_DEPTH_MIN,
					MINI_MART_SHELF_DEPTH_MAX,
					c.x,
					c.y,
					c.z,
					salt + 2.0,
				);
				let along_t = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, salt + 3.0);
				MiniMartShelfSpec {
					face,
					shelf: OptionalFaceBand {
						place: place_u < MINI_MART_SHELF_PLACE_RATE,
						along,
						depth,
						along_t,
					},
				}
			})
			.collect();

		Ok(Self {
			style,
			office_area_target,
			office_seed_depth,
			office_along_t,
			aisles_area_reserve,
			door_width,
			door_along_t,
			door_height,
			register_along_t,
			register_seed_depth,
			shelves,
		})
	}

	fn regions(&self) -> MiniMartRegions {
		MiniMartRegions {
			office_area_target: self.office_area_target,
			office_seed_depth: self.office_seed_depth,
			office_along_t: self.office_along_t,
			aisles_area_reserve: self.aisles_area_reserve,
			door_width: self.door_width,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			register_along_t: self.register_along_t,
			register_seed_depth: self.register_seed_depth,
			shelves: self.shelves.clone(),
		}
	}
}

/// Geometry resolved from [`MiniMartParameterized`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartPlan {
	pub parameterized: MiniMartParameterized,
	pub packed: MiniMartPacked,
}

impl MiniMartPlan {
	pub fn from_parameterized(
		params: MiniMartParameterized,
		confines: &Confines,
	) -> Result<Self, FitError> {
		let packed = params.regions().pack(confines)?;
		Ok(Self { parameterized: params, packed })
	}
}
