//! Parts plan packer: clearances → office enclosure (+door) → parts pockets.
//!
//! Smaller office/parts mins than MiniMart; office packing uses shared
//! [`super::enclosed_room`]. Authored door id scope: `parts_stall`.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use procedural_common::{
	aabb3_to_plan, clamp_min_size2, max_empty_rect2, plan_to_aabb3, Aabb2dPack, PlanAxes,
};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;

use crate::usage_areas::clearance::{PassageClearance, PASSAGE_CLEARANCE};

use super::enclosed_room::{EnclosedRoomMins, EnclosedRoomParams};

/// Scope prefix for [`OpeningId::scoped`] openings authored by Parts.
pub const SCOPE: &str = "parts_stall";

pub const PARTS_OFFICE_MIN: f32 = 2.0;
pub const PARTS_OFFICE_CONTACT: f32 = 2.0;
pub const PARTS_REGION_MIN: f32 = 2.0;
pub const PARTS_REGION_EXTRA_MIN: f32 = 1.5;
pub const PARTS_DOOR_WIDTH_MIN: f32 = 0.9 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const PARTS_DOOR_WIDTH_MAX: f32 = 1.2 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const PARTS_DOOR_HEIGHT_MIN: f32 = 2.0 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const PARTS_DOOR_HEIGHT_MAX: f32 = 2.4 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const PARTS_DOOR_HEADER_MIN: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct PartsOfficeDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartsRegions {
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	pub parts_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartsPacked {
	pub office: Aabb3d,
	pub parts: Vec<Aabb3d>,
	pub office_walls: Vec<Rectangle>,
	pub office_door: PartsOfficeDoor,
}

impl PartsRegions {
	pub fn pack(&self, confines: &Confines) -> Result<PartsPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall { reason: "parts passage" });
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let enclosed = EnclosedRoomParams {
			mins: EnclosedRoomMins::square(PARTS_OFFICE_MIN),
			contact: PARTS_OFFICE_CONTACT,
			seed_depth: self.office_seed_depth,
			along_t: self.office_along_t,
			area_target: self.office_area_target,
			area_reserve: self.parts_area_reserve.max(PARTS_REGION_MIN * PARTS_REGION_MIN),
			reserve_cap_frac: 0.7,
			grow_into: false,
			max_axis_frac: None,
			shrink_sales_for_door_clear: true,
			door_width: self.door_width,
			door_width_min: PARTS_DOOR_WIDTH_MIN,
			door_width_max: PARTS_DOOR_WIDTH_MAX,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			door_height_min: PARTS_DOOR_HEIGHT_MIN,
			door_height_max: PARTS_DOOR_HEIGHT_MAX,
			door_header_min: PARTS_DOOR_HEADER_MIN,
			door_clearance: PASSAGE_CLEARANCE,
			door_id: OpeningId::scoped(SCOPE, "office_door", "0"),
		}
		.pack(host3, host, &clearances)
		.ok_or(FitError::TooSmall { reason: "parts office" })?;
		crate::usage_areas::clearance::commit_door_clear(&mut clearances, enclosed.door_clear, 0.0);

		let parts = self
			.pack_parts(host, &clearances, enclosed.room)
			.ok_or(FitError::TooSmall { reason: "parts region" })?;

		Ok(PartsPacked {
			office: plan_to_aabb3(host3, enclosed.room, PlanAxes::XZ),
			parts: parts.into_iter().map(|p| plan_to_aabb3(host3, p, PlanAxes::XZ)).collect(),
			office_walls: enclosed.walls,
			office_door: PartsOfficeDoor { id: enclosed.door_id, opening: enclosed.door },
		})
	}

	fn pack_parts(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let primary = clamp_min_size2(grown, Vec2::splat(PARTS_REGION_MIN))?;
		let mut parts = vec![primary];
		hard.push(primary);
		for _ in 0..6 {
			let Some(seed) = max_empty_rect2(host, &hard) else {
				break;
			};
			let grown = seed.grow_into(host, &hard);
			let Some(extra) = clamp_min_size2(grown, Vec2::splat(PARTS_REGION_EXTRA_MIN)) else {
				break;
			};
			if !extra.is_clear_of(&hard) {
				break;
			}
			parts.push(extra);
			hard.push(extra);
		}
		Some(parts)
	}
}
