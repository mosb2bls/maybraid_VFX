//! Public restroom plan packer: strip reserve → walled stalls (+door) → sinks.
//!
//! Stalls seed against clearances∪door-side strip so they cannot eat the sink
//! zone; sinks use [`pack_abutting_clearance`] against the stalls-door keep-out.
//! Stall enclosure uses shared [`super::enclosed_room`]. Door id scope:
//! `public_restroom`.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use procedural_common::{aabb2_area, aabb3_to_plan, plan_to_aabb3, PlanAxes, PlanOpeningFace};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;
use crate::usage_areas::clearance::{
	abuts_clearance, pack_abutting_clearance, PassageClearance, PASSAGE_CLEARANCE,
};

use super::enclosed_room::{EnclosedRoomMins, EnclosedRoomParams};

/// Scope prefix for [`OpeningId::scoped`] openings authored by PublicRestroom.
pub const SCOPE: &str = "public_restroom";

pub const RESTROOM_STALLS_MIN: f32 = 2.0;
pub const RESTROOM_STALLS_CONTACT: f32 = 2.0;
pub const RESTROOM_SINK_MIN: f32 = 0.5;
pub const RESTROOM_SINK_DEPTH_MIN: f32 = 0.5;
pub const RESTROOM_SINK_DEPTH_MAX: f32 = 0.9;
pub const RESTROOM_DOOR_WIDTH_MIN: f32 = 0.9 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const RESTROOM_DOOR_WIDTH_MAX: f32 = 1.2 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const RESTROOM_DOOR_HEIGHT_MIN: f32 = 2.0 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const RESTROOM_DOOR_HEIGHT_MAX: f32 = 2.4 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const RESTROOM_DOOR_HEADER_MIN: f32 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct RestroomStallsDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomRegions {
	pub stalls_area_target: f32,
	pub stalls_seed_depth: f32,
	pub stalls_along_t: f32,
	/// Plan area reserved for sinks when clamping stalls grow (bites kitchen-style).
	pub sink_area_reserve: f32,
	/// Grow target for the primary sink pocket after seeding against the door clear.
	pub sink_area_target: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
	pub sink_depth: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroomPacked {
	pub stalls: Aabb3d,
	pub sinks: Vec<Aabb3d>,
	pub stall_walls: Vec<Rectangle>,
	pub stalls_door: RestroomStallsDoor,
}

impl PublicRestroomRegions {
	pub fn pack(&self, confines: &Confines) -> Result<PublicRestroomPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall { reason: "restroom passage" });
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let sink_depth = self.sink_depth.clamp(RESTROOM_SINK_DEPTH_MIN, RESTROOM_SINK_DEPTH_MAX);
		// Door-side strip toward the entry: stalls-door keep-out + sink pocket + customer
		// passage keep-out can stack on the same wall, so reserve all three.
		let free_strip_depth = 2.0 * PASSAGE_CLEARANCE + sink_depth;
		let strip_area = free_strip_depth * (host.max.x - host.min.x).max(host.max.y - host.min.y);
		let area_reserve = self
			.sink_area_reserve
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.max(strip_area * 0.5);

		let enclosed = EnclosedRoomParams {
			mins: EnclosedRoomMins::square(RESTROOM_STALLS_MIN),
			contact: RESTROOM_STALLS_CONTACT,
			seed_depth: self.stalls_seed_depth,
			along_t: self.stalls_along_t,
			area_target: self.stalls_area_target,
			area_reserve,
			reserve_cap_frac: 0.35,
			grow_into: true,
			max_axis_frac: None,
			shrink_sales_for_door_clear: false,
			door_width: self.door_width,
			door_width_min: RESTROOM_DOOR_WIDTH_MIN,
			door_width_max: RESTROOM_DOOR_WIDTH_MAX,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			door_height_min: RESTROOM_DOOR_HEIGHT_MIN,
			door_height_max: RESTROOM_DOOR_HEIGHT_MAX,
			door_header_min: RESTROOM_DOOR_HEADER_MIN,
			door_clearance: PASSAGE_CLEARANCE,
			door_id: OpeningId::scoped(SCOPE, "stalls_door", "0"),
		}
		.pack_filtered(
			host3,
			host,
			&clearances,
			|face| Self::free_strip_block(host, face, free_strip_depth).map(|strip| vec![strip]),
			|stalls, face| {
				let Some(zone) = Self::door_side_zone(host, stalls, face) else {
					return false;
				};
				let zone_short = (zone.max.x - zone.min.x).min(zone.max.y - zone.min.y);
				zone_short + 1e-3 >= RESTROOM_SINK_MIN
			},
		)
		.ok_or(FitError::TooSmall { reason: "restroom stalls" })?;
		crate::usage_areas::clearance::commit_door_clear(&mut clearances, enclosed.door_clear, 0.0);

		let sinks = self
			.pack_sinks(host, &clearances, enclosed.room, enclosed.seed_face, enclosed.door_clear)
			.ok_or(FitError::TooSmall { reason: "restroom sink" })?;

		Ok(PublicRestroomPacked {
			stalls: plan_to_aabb3(host3, enclosed.room, PlanAxes::XZ),
			sinks: sinks.into_iter().map(|s| plan_to_aabb3(host3, s, PlanAxes::XZ)).collect(),
			stall_walls: enclosed.walls,
			stalls_door: RestroomStallsDoor { id: enclosed.door_id, opening: enclosed.door },
		})
	}

	/// Free strip on the door side of stalls (between stalls free edge and opposite host wall).
	fn door_side_zone(host: Aabb2d, stalls2: Aabb2d, seed_face: PlanOpeningFace) -> Option<Aabb2d> {
		const EPS: f32 = 1e-3;
		let zone = if seed_face.thru_is_x {
			if seed_face.inward_positive {
				Aabb2d {
					min: Vec2::new(stalls2.max.x + EPS, host.min.y),
					max: Vec2::new(host.max.x, host.max.y),
				}
			} else {
				Aabb2d {
					min: Vec2::new(host.min.x, host.min.y),
					max: Vec2::new(stalls2.min.x - EPS, host.max.y),
				}
			}
		} else if seed_face.inward_positive {
			Aabb2d {
				min: Vec2::new(host.min.x, stalls2.max.y + EPS),
				max: Vec2::new(host.max.x, host.max.y),
			}
		} else {
			Aabb2d {
				min: Vec2::new(host.min.x, host.min.y),
				max: Vec2::new(host.max.x, stalls2.min.y - EPS),
			}
		};
		if zone.max.x - zone.min.x < RESTROOM_SINK_MIN - 1e-3
			|| zone.max.y - zone.min.y < RESTROOM_SINK_MIN - 1e-3
		{
			return None;
		}
		Some(zone)
	}

	/// Inward band from the host wall opposite `seed_face`, reserved for door clearance + sinks.
	fn free_strip_block(host: Aabb2d, seed_face: PlanOpeningFace, depth: f32) -> Option<Aabb2d> {
		let opposite = PlanOpeningFace {
			thru_is_x: seed_face.thru_is_x,
			thru: if seed_face.thru_is_x {
				if seed_face.inward_positive {
					host.max.x
				} else {
					host.min.x
				}
			} else if seed_face.inward_positive {
				host.max.y
			} else {
				host.min.y
			},
			along0: if seed_face.thru_is_x { host.min.y } else { host.min.x },
			along1: if seed_face.thru_is_x { host.max.y } else { host.max.x },
			inward_positive: !seed_face.inward_positive,
		};
		opposite.band(host, opposite.along_len(), depth, 0.5)
	}

	/// Sinks live in the door-side free strip, abutting the stalls-door clearance
	/// (kitchen-style: seed against keep-out, then grow toward a target).
	fn pack_sinks(
		&self,
		_host: Aabb2d,
		clearances: &[Aabb2d],
		stalls2: Aabb2d,
		seed_face: PlanOpeningFace,
		door_clear: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let zone = Self::door_side_zone(_host, stalls2, seed_face)?;
		let mut hard = clearances.to_vec();
		hard.push(stalls2);

		let min = Vec2::splat(RESTROOM_SINK_MIN);
		let zone_area = aabb2_area(zone);
		let target = self
			.sink_area_target
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.min(zone_area.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN));

		let primary = pack_abutting_clearance(zone, &hard, door_clear, min, target)?;
		debug_assert!(abuts_clearance(primary, door_clear));
		let mut sinks = vec![primary];
		hard.push(primary);

		// Opportunistic second pocket (same abutting grow) if space remains.
		let extra_target = (target * 0.55)
			.max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN)
			.min((zone_area - aabb2_area(primary)).max(RESTROOM_SINK_MIN * RESTROOM_SINK_MIN));
		if let Some(extra) = pack_abutting_clearance(zone, &hard, door_clear, min, extra_target) {
			sinks.push(extra);
		}

		Some(sinks)
	}
}
