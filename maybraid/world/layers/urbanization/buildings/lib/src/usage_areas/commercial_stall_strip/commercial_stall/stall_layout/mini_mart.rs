//! MiniMart plan packer: clearances → office enclosure → register → aisles → shelves.
//!
//! Owns office-door [`OpeningId`] scope (`mini_mart`) and mins for each stage.
//! Office packing uses shared [`super::enclosed_room`]. Shelves are optional wall
//! bands on host faces free of customer passages.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use procedural_common::{
	aabb3_to_plan, clamp_min_size2, intersects_aabb2, max_empty_rect2, plan_to_aabb3, Aabb2dPack,
	OptionalFaceBand, PlanAxes, PlanOpeningFace,
};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;

use crate::usage_areas::clearance::{PassageClearance, PlanHost, PASSAGE_CLEARANCE};

use super::enclosed_room::{EnclosedRoomMins, EnclosedRoomParams};

/// Scope prefix for [`OpeningId::scoped`] openings authored by MiniMart.
pub const SCOPE: &str = "mini_mart";

/// Inward clearance kept free in front of every customer passage (and office door).
pub const MINI_MART_PASSAGE_CLEARANCE: f32 = PASSAGE_CLEARANCE;
/// Office: longer plan axis must be at least this.
pub const MINI_MART_OFFICE_LONG_MIN: f32 = 3.0;
/// Office: shorter plan axis must be at least this.
pub const MINI_MART_OFFICE_SHORT_MIN: f32 = 2.0;
/// Register plan minimum (both axes).
pub const MINI_MART_REGISTER_MIN: f32 = 2.0;
/// Primary aisles plan minimum (both axes) — at least one region must meet this.
pub const MINI_MART_AISLES_MIN: f32 = 4.0;
/// Extra discontiguous aisle pockets may be this small (both axes).
pub const MINI_MART_AISLES_EXTRA_MIN: f32 = 2.0;
/// Shelf depth sample / pack range.
pub const MINI_MART_SHELF_DEPTH_MIN: f32 = 0.5;
pub const MINI_MART_SHELF_DEPTH_MAX: f32 = 1.0;
/// Minimum along-length for an opportunistic shelf segment.
pub const MINI_MART_SHELF_ALONG_MIN: f32 = 0.75;
/// Default place-rate for optional wall shelves.
pub const MINI_MART_SHELF_PLACE_RATE: f32 = 0.55;
/// Office door along-width range.
pub const MINI_MART_DOOR_WIDTH_MIN: f32 = 0.9 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const MINI_MART_DOOR_WIDTH_MAX: f32 = 1.2 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
/// Office door clear opening height (leaves a header to the ceiling when host is taller).
pub const MINI_MART_DOOR_HEIGHT_MIN: f32 = 2.0 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
pub const MINI_MART_DOOR_HEIGHT_MAX: f32 = 2.4 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
/// Minimum header band above the office door.
pub const MINI_MART_DOOR_HEADER_MIN: f32 = 0.25;
/// Minimum office face contact when seeding.
pub const MINI_MART_OFFICE_CONTACT: f32 = 2.0;

/// Per free-wall shelf choice.
pub type MiniMartShelfChoice = OptionalFaceBand;

/// Free host wall snapshotted with its shelf choice.
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartShelfSpec {
	pub face: PlanOpeningFace,
	pub shelf: MiniMartShelfChoice,
}

/// Noise knobs consumed by [`MiniMartRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartRegions {
	pub office_area_target: f32,
	pub office_seed_depth: f32,
	pub office_along_t: f32,
	/// Plan area reserved for aisles (+ register floor) when clamping office grow.
	pub aisles_area_reserve: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	/// Clear opening height for the office door (m); header fills to the stall ceiling.
	pub door_height: f32,
	pub register_along_t: f32,
	pub register_seed_depth: f32,
	pub shelves: Vec<MiniMartShelfSpec>,
}

/// Authored office-door passage + enclosure panels.
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartOfficeDoor {
	pub id: OpeningId,
	pub opening: Opening,
}

/// Geometry produced by [`MiniMartRegions::pack`].
#[derive(Debug, Clone, PartialEq)]
pub struct MiniMartPacked {
	pub office: Aabb3d,
	pub register: Aabb3d,
	/// One or more aisle pockets; the first meets [`MINI_MART_AISLES_MIN`].
	pub aisles: Vec<Aabb3d>,
	pub shelves: Vec<Aabb3d>,
	pub office_walls: Vec<Rectangle>,
	/// Passage through the office sales divider (tracked id + void bounds).
	pub office_door: MiniMartOfficeDoor,
}

impl MiniMartRegions {
	pub fn pack(&self, confines: &Confines) -> Result<MiniMartPacked, FitError> {
		let host3 = &confines.bounds;
		let host = aabb3_to_plan(host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		if passage_faces.is_empty() {
			return Err(FitError::TooSmall { reason: "mini mart passage" });
		}

		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let sales_floor = (MINI_MART_AISLES_MIN * MINI_MART_AISLES_MIN)
			+ (MINI_MART_REGISTER_MIN * MINI_MART_REGISTER_MIN);
		let enclosed = EnclosedRoomParams {
			mins: EnclosedRoomMins {
				long: MINI_MART_OFFICE_LONG_MIN,
				short: MINI_MART_OFFICE_SHORT_MIN,
			},
			contact: MINI_MART_OFFICE_CONTACT,
			seed_depth: self.office_seed_depth,
			along_t: self.office_along_t,
			area_target: self.office_area_target,
			area_reserve: self.aisles_area_reserve.max(sales_floor),
			reserve_cap_frac: 0.75,
			grow_into: false,
			max_axis_frac: None,
			shrink_sales_for_door_clear: true,
			door_width: self.door_width,
			door_width_min: MINI_MART_DOOR_WIDTH_MIN,
			door_width_max: MINI_MART_DOOR_WIDTH_MAX,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			door_height_min: MINI_MART_DOOR_HEIGHT_MIN,
			door_height_max: MINI_MART_DOOR_HEIGHT_MAX,
			door_header_min: MINI_MART_DOOR_HEADER_MIN,
			door_clearance: MINI_MART_PASSAGE_CLEARANCE,
			door_id: OpeningId::scoped(SCOPE, "office_door", "0"),
		}
		.pack(host3, host, &clearances)
		.ok_or(FitError::TooSmall { reason: "mini mart office" })?;
		crate::usage_areas::clearance::commit_door_clear(&mut clearances, enclosed.door_clear, 0.0);
		let office2 = enclosed.room;

		let register2 = self
			.pack_register(host, &passage_faces, &clearances, office2)
			.ok_or(FitError::TooSmall { reason: "mini mart register" })?;

		let aisles2 = self
			.pack_aisles(host, &clearances, office2, register2)
			.ok_or(FitError::TooSmall { reason: "mini mart aisles" })?;

		let shelves = self.pack_shelves(host, &clearances, office2, register2, &aisles2);

		Ok(MiniMartPacked {
			office: plan_to_aabb3(host3, office2, PlanAxes::XZ),
			register: plan_to_aabb3(host3, register2, PlanAxes::XZ),
			aisles: aisles2.into_iter().map(|a| plan_to_aabb3(host3, a, PlanAxes::XZ)).collect(),
			shelves: shelves.into_iter().map(|s| plan_to_aabb3(host3, s, PlanAxes::XZ)).collect(),
			office_walls: enclosed.walls,
			office_door: MiniMartOfficeDoor { id: enclosed.door_id, opening: enclosed.door },
		})
	}

	fn pack_register(
		&self,
		host: Aabb2d,
		passage_faces: &[PlanOpeningFace],
		clearances: &[Aabb2d],
		office2: Aabb2d,
	) -> Option<Aabb2d> {
		let min = MINI_MART_REGISTER_MIN;
		let depth = self.register_seed_depth.max(min);
		// Passage clearances abut the inset seed face — use them only as open-overlap
		// excludes (not free-segment blockers on that face).
		let mut hard = clearances.to_vec();
		hard.push(office2);

		let mut order: Vec<usize> = (0..passage_faces.len()).collect();
		order.sort_by(|a, b| {
			passage_faces[*b]
				.along_len()
				.partial_cmp(&passage_faces[*a].along_len())
				.unwrap_or(std::cmp::Ordering::Equal)
		});

		for &i in &order {
			let face = passage_faces[i];
			let inset = inset_face(face, MINI_MART_PASSAGE_CLEARANCE);
			for &t in &[self.register_along_t, 0.5, 0.0, 1.0] {
				let Some(seed) = inset.seed(host, min, depth, t) else {
					continue;
				};
				if !seed.is_clear_of(&hard) {
					continue;
				}
				let grown = seed.grow_toward_area(host, &hard, min * min);
				let Some(reg) = clamp_min_size2(grown, Vec2::splat(min)) else {
					continue;
				};
				if !reg.is_clear_of(&hard) {
					continue;
				}
				return Some(reg);
			}
		}

		// Fallback: largest empty ≥2×2 in the sales floor.
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let reg = clamp_min_size2(grown, Vec2::splat(min))?;
		reg.is_clear_of(&hard).then_some(reg)
	}

	fn pack_aisles(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
		register2: Aabb2d,
	) -> Option<Vec<Aabb2d>> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		hard.push(register2);
		let seed = max_empty_rect2(host, &hard)?;
		let grown = seed.grow_into(host, &hard);
		let primary = clamp_min_size2(grown, Vec2::splat(MINI_MART_AISLES_MIN))?;
		let mut aisles = vec![primary];
		hard.push(primary);

		// Fill leftover pockets with smaller discontiguous aisle regions.
		for _ in 0..6 {
			let Some(seed) = max_empty_rect2(host, &hard) else {
				break;
			};
			let grown = seed.grow_into(host, &hard);
			let Some(extra) = clamp_min_size2(grown, Vec2::splat(MINI_MART_AISLES_EXTRA_MIN))
			else {
				break;
			};
			if !extra.is_clear_of(&hard) {
				break;
			}
			aisles.push(extra);
			hard.push(extra);
		}
		Some(aisles)
	}

	fn pack_shelves(
		&self,
		host: Aabb2d,
		clearances: &[Aabb2d],
		office2: Aabb2d,
		register2: Aabb2d,
		aisles: &[Aabb2d],
	) -> Vec<Aabb2d> {
		let mut hard = clearances.to_vec();
		hard.push(office2);
		hard.push(register2);
		hard.extend_from_slice(aisles);
		let mut out = Vec::new();

		// Sampled shelf choices first.
		for spec in &self.shelves {
			let depth =
				spec.shelf.depth.clamp(MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_DEPTH_MAX);
			let choice = OptionalFaceBand {
				place: spec.shelf.place,
				along: spec.shelf.along,
				depth,
				along_t: spec.shelf.along_t,
			};
			let Some(band) = choice.resolve(host, spec.face) else {
				continue;
			};
			if !band.is_clear_of(&hard) {
				continue;
			}
			if out.iter().any(|s| intersects_aabb2(band, *s)) {
				continue;
			}
			out.push(band);
			hard.push(band);
		}

		// Opportunistic fill: more discontiguous shelves on free-wall free segments.
		let free_faces: Vec<PlanOpeningFace> = self.shelves.iter().map(|s| s.face).collect();
		for face in free_faces {
			let depth = self
				.shelves
				.iter()
				.find(|s| PlanHost::same_wall(s.face, face))
				.map(|s| s.shelf.depth.clamp(MINI_MART_SHELF_DEPTH_MIN, MINI_MART_SHELF_DEPTH_MAX))
				.unwrap_or(0.75);
			// Recompute free segments as shelves are accepted.
			for _ in 0..8 {
				let Some((seg0, seg1)) =
					face.longest_free_segment(&hard, MINI_MART_SHELF_ALONG_MIN)
				else {
					break;
				};
				let avail = seg1 - seg0;
				let seg_face = PlanOpeningFace { along0: seg0, along1: seg1, ..face };
				let Some(band) = seg_face.band(host, avail, depth, 0.5) else {
					break;
				};
				if !band.is_clear_of(&hard) {
					break;
				}
				if out.iter().any(|s| intersects_aabb2(band, *s)) {
					break;
				}
				out.push(band);
				hard.push(band);
			}
		}
		out
	}
}

fn inset_face(face: PlanOpeningFace, depth: f32) -> PlanOpeningFace {
	let thru = if face.inward_positive { face.thru + depth } else { face.thru - depth };
	PlanOpeningFace { thru, ..face }
}
