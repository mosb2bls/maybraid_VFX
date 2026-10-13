//! Plan packing for [`super::CommonBedroom`]: passage clearances, beds /
//! nightstands, then wall-seeded closets and ensuites via [`EnclosedRoom`].
//!
//! Kind selection uses the shared [`crate::placer`] catalog model
//! ([`KindSpec`], [`pick_kind`], [`OccupiedBudget`]). Bed / nightstand adjacency
//! and ensuite `max_axis_frac: 0.5` stay bedroom-local in `try_place`.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{
	aabb2_area, aabb3_to_plan, inflate_aabb2, intersects_aabb2, plan_to_aabb3, touches_aabb2,
	NoiseConfig, NoiseParams, PlanAxes,
};

use crate::fit::{Confines, FitError};
use crate::openings::{Opening, OpeningId};
use crate::paneling::Rectangle;
use crate::placer::{
	enclosure_soft_goal_met, pick_kind, propose_from_spec, try_free_extent, CommitEffect,
	FreeExtentKnobs, KindSpec, OccupiedBudget, Predicate, ProgramTier, ProposeKnobs, SoftGoalRole,
};
use crate::usage_areas::clearance::{commit_door_clear, PassageClearance, PASSAGE_APPROACH_PAD};
use crate::usage_areas::enclosed_room::{EnclosedRoom, EnclosedRoomMins, EnclosedRoomParams};

use super::parameterized::SCOPE;

const DOOR_WIDTH_MIN: f32 = 0.65 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
const DOOR_WIDTH_MAX: f32 = 1.15 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
const DOOR_HEIGHT_MIN: f32 = 1.8 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
const DOOR_HEIGHT_MAX: f32 = 2.3 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
const DOOR_HEADER_MIN: f32 = 0.2;
/// Plan pad kept between closet↔closet and closet↔ensuite (avoids thin wall slivers).
const PARTITION_SEP: f32 = 1.0;
/// Plan pad between free-standing storage (wardrobe↔dresser / same-kind pairs).
const STORAGE_SEP: f32 = 1.0;
/// Inward door keep-out depth for closet / walk-in / ensuite sales-face doors.
const PARTITION_DOOR_CLEARANCE: f32 = 1.25;
/// Host floor area (m²) above which walk-ins / bedroom furniture may be considered.
const LARGE_ROOM_AREA: f32 = 70.0;
/// Soft cap on residual bedroom-furniture placements.
const MAX_BEDROOM_FURNITURE: usize = 1;
/// Soft cap on shallow closets (walk-in / ensuite are separately unique).
const MAX_CLOSETS: usize = 2;
const WALL_EPS: f32 = 0.06;
const PACK_STEPS: u32 = 24;

/// One closet or ensuite: enclosure panels + authored door + room AABB.
#[derive(Debug, Clone, PartialEq)]
pub struct BedroomPartition {
	pub bounds: Aabb3d,
	pub walls: Vec<Rectangle>,
	pub door_id: OpeningId,
	pub door: Opening,
	pub door_clear: Aabb2d,
}

/// Packed bedroom program inside host confines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CommonBedroomPacked {
	pub beds: Vec<Aabb3d>,
	/// Small boxes placed adjacent to a bed.
	pub nightstands: Vec<Aabb3d>,
	/// Same scale as nightstands, but not bed-adjacent (chests / stools / …).
	pub small_bedroom_furniture: Vec<Aabb3d>,
	/// Free-standing tall storage (not inside a closet cell).
	pub wardrobes: Vec<Aabb3d>,
	/// Free-standing low storage.
	pub dressers: Vec<Aabb3d>,
	/// Mid-size free furniture for roomy hosts.
	pub bedroom_furniture: Vec<Aabb3d>,
	pub closets: Vec<BedroomPartition>,
	pub walk_in_closets: Vec<BedroomPartition>,
	pub ensuites: Vec<BedroomPartition>,
}

/// Runtime knobs used by [`CommonBedroomRegions::pack`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommonBedroomRegions {
	pub spaciousness: f32,
	pub occupancy: f32,
	pub bed_against_wall: bool,
	pub ensuite_area_target: f32,
	pub bedroom_area_reserve: f32,
	pub walk_in_area_target: f32,
	pub closet_along_t: f32,
	pub walk_in_along_t: f32,
	pub ensuite_along_t: f32,
	pub door_width: f32,
	pub door_along_t: f32,
	pub door_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BedroomKind {
	Bed,
	Nightstand,
	Wardrobe,
	Dresser,
	BedroomFurniture,
	Closet,
	WalkInCloset,
	Ensuite,
}

enum Placed {
	Solid(Aabb3d),
	Nightstand(Aabb3d),
	SmallFurniture(Aabb3d),
	Wardrobe(Aabb3d),
	Dresser(Aabb3d),
	BedroomFurniture(Aabb3d),
	Closet(BedroomPartition),
	WalkIn(BedroomPartition),
	Ensuite(BedroomPartition),
}

impl CommonBedroomRegions {
	fn catalog() -> &'static [KindSpec<BedroomKind>] {
		static CATALOG: [KindSpec<BedroomKind>; 8] = [
			KindSpec {
				id: BedroomKind::Ensuite,
				tier: ProgramTier::Enclosure,
				weight: 0.28,
				max_count: Some(1),
				soft_goal: SoftGoalRole::Ensuite,
				propose: ProposeKnobs::EnclosedRoom,
				predicates: &[],
				commit: CommitEffect::WalledWithDoor { door_approach_pad: PASSAGE_APPROACH_PAD },
				structure_budget: true,
			},
			KindSpec {
				id: BedroomKind::WalkInCloset,
				tier: ProgramTier::Enclosure,
				weight: 0.14,
				max_count: Some(1),
				soft_goal: SoftGoalRole::ClosetLike,
				propose: ProposeKnobs::EnclosedRoom,
				predicates: &[],
				commit: CommitEffect::WalledWithDoor { door_approach_pad: PASSAGE_APPROACH_PAD },
				structure_budget: true,
			},
			KindSpec {
				id: BedroomKind::Closet,
				tier: ProgramTier::Enclosure,
				weight: 0.58,
				max_count: Some(MAX_CLOSETS),
				soft_goal: SoftGoalRole::ClosetLike,
				propose: ProposeKnobs::EnclosedRoom,
				predicates: &[],
				commit: CommitEffect::WalledWithDoor { door_approach_pad: PASSAGE_APPROACH_PAD },
				structure_budget: true,
			},
			KindSpec {
				id: BedroomKind::Nightstand,
				tier: ProgramTier::Appointed,
				weight: 0.28,
				max_count: None,
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 0.45,
					max_x: 0.45,
					min_z: 0.45,
					max_z: 0.45,
					height: 0.5,
					prefer_wall: false,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: BedroomKind::Bed,
				tier: ProgramTier::Appointed,
				weight: 0.14,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 2.0,
					max_x: 2.0,
					min_z: 1.6,
					max_z: 1.6,
					height: 0.55,
					prefer_wall: true,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: BedroomKind::Wardrobe,
				tier: ProgramTier::Appointed,
				weight: 0.12,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 1.1,
					along_max: 1.1,
					depth_min: 0.6,
					depth_max: 0.6,
					height: 2.1,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: BedroomKind::Dresser,
				tier: ProgramTier::Appointed,
				weight: 0.10,
				max_count: Some(1),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::WallLong {
					along_min: 1.3,
					along_max: 1.3,
					depth_min: 0.5,
					depth_max: 0.5,
					height: 0.9,
				},
				predicates: &[
					Predicate::InHost,
					Predicate::ClearOfKeepOuts,
					Predicate::LongFaceOnWall,
				],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
			KindSpec {
				id: BedroomKind::BedroomFurniture,
				tier: ProgramTier::Filler,
				weight: 0.14,
				max_count: Some(MAX_BEDROOM_FURNITURE),
				soft_goal: SoftGoalRole::None,
				propose: ProposeKnobs::FreeExtent {
					min_x: 1.5,
					max_x: 1.5,
					min_z: 0.85,
					max_z: 0.85,
					height: 0.9,
					prefer_wall: false,
				},
				predicates: &[Predicate::InHost, Predicate::ClearOfKeepOuts],
				commit: CommitEffect::SolidFootprint,
				structure_budget: false,
			},
		];
		&CATALOG
	}

	pub fn pack(
		&self,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<CommonBedroomPacked, FitError> {
		let host3 = confines.bounds;
		let host = aabb3_to_plan(&host3, PlanAxes::XZ);
		let passage_faces = PassageClearance::collect_faces(confines, host);
		let mut clearances = PassageClearance::bands_std(host, &passage_faces);
		let room_area = aabb2_area(host).max(1e-4);
		let cfg = NoiseConfig::new(noise);

		let mut packed = CommonBedroomPacked::default();
		let mut budget = OccupiedBudget::new(room_area, self.occupancy, self.occupancy.max(0.78));

		if let Some(bed) = place_bed(
			&host3,
			host,
			&clearances,
			&packed,
			&cfg,
			0,
			self.spaciousness,
			self.bed_against_wall,
		) {
			budget.commit(xz_area(&bed));
			clearances.push(aabb3_to_plan(&bed, PlanAxes::XZ));
			packed.beds.push(bed);
		} else {
			return Err(FitError::TooSmall { reason: "common bedroom bed" });
		}

		for step in 1..PACK_STEPS {
			if budget.furniture_full() {
				break;
			}
			let soft_goal = enclosure_soft_goal_met(
				!packed.ensuites.is_empty(),
				packed.closets.len() + packed.walk_in_closets.len(),
			);
			let eligible =
				eligible_catalog(Self::catalog(), soft_goal, room_area, step, packed.beds.len());
			let Some(kind) =
				pick_kind(&eligible, &cfg, step, soft_goal, |k| count_kind(&packed, k))
			else {
				continue;
			};
			let Some(placed) = self.try_place(kind, &host3, host, &clearances, &packed, &cfg, step)
			else {
				continue;
			};
			let (add, is_structure) = placed_area(&placed);
			if !budget.accepts(add, is_structure) {
				continue;
			}
			budget.commit(add);
			commit_placed(placed, &mut clearances, &mut packed);
		}

		Ok(packed)
	}

	fn try_place(
		&self,
		kind: BedroomKind,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		packed: &CommonBedroomPacked,
		cfg: &NoiseConfig,
		salt: u32,
	) -> Option<Placed> {
		match kind {
			BedroomKind::Bed => place_bed(
				host3,
				host,
				clearances,
				packed,
				cfg,
				salt,
				self.spaciousness,
				self.bed_against_wall,
			)
			.map(Placed::Solid),
			BedroomKind::Nightstand => {
				place_small_box(host3, host, clearances, packed, cfg, salt, self.spaciousness)
			}
			BedroomKind::Wardrobe => self
				.try_place_catalog_solid(
					BedroomKind::Wardrobe,
					host3,
					host,
					&storage_pack_clearances(clearances, packed),
					packed,
					cfg,
					salt,
				)
				.map(Placed::Wardrobe),
			BedroomKind::Dresser => self
				.try_place_catalog_solid(
					BedroomKind::Dresser,
					host3,
					host,
					&storage_pack_clearances(clearances, packed),
					packed,
					cfg,
					salt,
				)
				.map(Placed::Dresser),
			BedroomKind::BedroomFurniture => self
				.try_place_catalog_solid(
					BedroomKind::BedroomFurniture,
					host3,
					host,
					clearances,
					packed,
					cfg,
					salt,
				)
				.map(Placed::BedroomFurniture),
			BedroomKind::Closet => {
				self.try_place_closet(host3, host, clearances, packed).map(Placed::Closet)
			}
			BedroomKind::WalkInCloset => {
				if !packed.walk_in_closets.is_empty() {
					return None;
				}
				let clears = partition_pack_clearances(clearances, packed);
				let mins = self.walk_in_mins();
				if let Some(part) = self.pack_partition(
					host3,
					host,
					&clears,
					mins,
					self.walk_in_area_target.max(mins.long * mins.short),
					self.bedroom_floor_reserve(host),
					self.walk_in_along_t,
					OpeningId::scoped(SCOPE, "walk_in_door", "0"),
				) {
					return Some(Placed::WalkIn(part));
				}
				self.try_place_closet(host3, host, clearances, packed).map(Placed::Closet)
			}
			BedroomKind::Ensuite => {
				if !packed.ensuites.is_empty() {
					return None;
				}
				let clears = partition_pack_clearances(clearances, packed);
				let mins = self.ensuite_mins();
				let usable = aabb2_area(host);
				let area_target = self
					.ensuite_area_target
					.max(mins.long * mins.short)
					.max(usable * 0.35)
					.min(usable * 0.50);
				let area_reserve = self
					.bedroom_area_reserve
					.min(usable * 0.55)
					.max(self.bedroom_floor_reserve(host).min(usable * 0.45));
				if let Some(part) = self.pack_partition_with(
					host3,
					host,
					&clears,
					mins,
					area_target,
					area_reserve,
					self.ensuite_along_t,
					OpeningId::scoped(SCOPE, "ensuite_door", "0"),
					0.55,
					true,
					Some(0.5),
				) {
					return Some(Placed::Ensuite(part));
				}
				self.try_place_closet(host3, host, clearances, packed).map(Placed::Closet)
			}
		}
	}

	/// Wardrobe / dresser / filler solids via catalog [`propose_from_spec`].
	fn try_place_catalog_solid(
		&self,
		kind: BedroomKind,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		packed: &CommonBedroomPacked,
		cfg: &NoiseConfig,
		salt: u32,
	) -> Option<Aabb3d> {
		let spec = Self::catalog().iter().find(|s| s.id == kind)?;
		let center = Vec3::new(
			(host.min.x + host.max.x) * 0.5,
			host3.min.y,
			(host.min.y + host.max.y) * 0.5,
		);
		let candidate =
			propose_from_spec(spec, self.spaciousness, host3, host, clearances, cfg, salt, center)?;
		if collides_solids(&candidate, packed) {
			return None;
		}
		Some(candidate)
	}

	fn try_place_closet(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		packed: &CommonBedroomPacked,
	) -> Option<BedroomPartition> {
		if packed.closets.len() >= MAX_CLOSETS {
			return None;
		}
		let clears = partition_pack_clearances(clearances, packed);
		let mins = self.closet_mins();
		let area_target = (mins.long * mins.short * 1.15).min(aabb2_area(host) * 0.18);
		self.pack_partition(
			host3,
			host,
			&clears,
			mins,
			area_target,
			self.bedroom_floor_reserve(host),
			self.closet_along_t,
			OpeningId::scoped(SCOPE, "closet_door", packed.closets.len().to_string()),
		)
	}

	fn closet_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_closet_length(self.spaciousness),
			short: base_closet_depth(self.spaciousness),
		}
	}

	fn walk_in_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_walk_in_length(self.spaciousness),
			short: base_walk_in_depth(self.spaciousness),
		}
	}

	fn ensuite_mins(&self) -> EnclosedRoomMins {
		EnclosedRoomMins {
			long: base_ensuite_length(self.spaciousness),
			short: base_ensuite_depth(self.spaciousness),
		}
	}

	fn bedroom_floor_reserve(&self, host: Aabb2d) -> f32 {
		let bed_floor = 2.0 * 1.6 * self.spaciousness * self.spaciousness;
		bed_floor.max(aabb2_area(host) * 0.28)
	}

	fn pack_partition(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mins: EnclosedRoomMins,
		area_target: f32,
		area_reserve: f32,
		along_t: f32,
		door_id: OpeningId,
	) -> Option<BedroomPartition> {
		self.pack_partition_with(
			host3,
			host,
			clearances,
			mins,
			area_target,
			area_reserve,
			along_t,
			door_id,
			0.72,
			false,
			None,
		)
	}

	fn pack_partition_with(
		&self,
		host3: &Aabb3d,
		host: Aabb2d,
		clearances: &[Aabb2d],
		mins: EnclosedRoomMins,
		area_target: f32,
		area_reserve: f32,
		along_t: f32,
		door_id: OpeningId,
		reserve_cap_frac: f32,
		grow_into: bool,
		max_axis_frac: Option<f32>,
	) -> Option<BedroomPartition> {
		let contact = mins.long.max(0.55);
		let enclosed = EnclosedRoomParams {
			mins,
			contact,
			seed_depth: mins.short.max(0.6),
			along_t,
			area_target,
			area_reserve,
			reserve_cap_frac,
			grow_into,
			max_axis_frac,
			shrink_sales_for_door_clear: true,
			door_width: self.door_width,
			door_width_min: DOOR_WIDTH_MIN,
			door_width_max: DOOR_WIDTH_MAX,
			door_along_t: self.door_along_t,
			door_height: self.door_height,
			door_height_min: DOOR_HEIGHT_MIN,
			door_height_max: DOOR_HEIGHT_MAX,
			door_header_min: DOOR_HEADER_MIN,
			door_clearance: PARTITION_DOOR_CLEARANCE,
			door_id: door_id.clone(),
		}
		.pack(host3, host, clearances)?;

		Some(partition_from_enclosed(host3, enclosed))
	}
}

fn eligible_catalog(
	catalog: &[KindSpec<BedroomKind>],
	soft_goal_met: bool,
	room_area: f32,
	step: u32,
	bed_count: usize,
) -> Vec<KindSpec<BedroomKind>> {
	let large = room_area + 1e-3 >= LARGE_ROOM_AREA;
	catalog
		.iter()
		.filter_map(|spec| {
			if !soft_goal_met && spec.tier != ProgramTier::Enclosure {
				return None;
			}
			if spec.id == BedroomKind::WalkInCloset && !large {
				return None;
			}
			if spec.id == BedroomKind::BedroomFurniture && (!large || step < 8) {
				return None;
			}
			if spec.id == BedroomKind::Bed && bed_count >= 1 {
				return None;
			}
			Some(spec.clone())
		})
		.collect()
}

fn count_kind(packed: &CommonBedroomPacked, kind: BedroomKind) -> usize {
	match kind {
		BedroomKind::Bed => packed.beds.len(),
		BedroomKind::Nightstand => packed.nightstands.len(),
		BedroomKind::Wardrobe => packed.wardrobes.len(),
		BedroomKind::Dresser => packed.dressers.len(),
		BedroomKind::BedroomFurniture => packed.bedroom_furniture.len(),
		BedroomKind::Closet => packed.closets.len(),
		BedroomKind::WalkInCloset => packed.walk_in_closets.len(),
		BedroomKind::Ensuite => packed.ensuites.len(),
	}
}

fn placed_area(placed: &Placed) -> (f32, bool) {
	match placed {
		Placed::Solid(a)
		| Placed::Nightstand(a)
		| Placed::SmallFurniture(a)
		| Placed::Wardrobe(a)
		| Placed::Dresser(a)
		| Placed::BedroomFurniture(a) => (xz_area(a), false),
		Placed::Closet(p) | Placed::WalkIn(p) | Placed::Ensuite(p) => (xz_area(&p.bounds), true),
	}
}

fn commit_placed(placed: Placed, clearances: &mut Vec<Aabb2d>, packed: &mut CommonBedroomPacked) {
	match placed {
		Placed::Solid(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.beds.push(a);
		}
		Placed::Nightstand(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.nightstands.push(a);
		}
		Placed::SmallFurniture(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.small_bedroom_furniture.push(a);
		}
		Placed::Wardrobe(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.wardrobes.push(a);
		}
		Placed::Dresser(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.dressers.push(a);
		}
		Placed::BedroomFurniture(a) => {
			clearances.push(aabb3_to_plan(&a, PlanAxes::XZ));
			packed.bedroom_furniture.push(a);
		}
		Placed::Closet(part) => {
			clearances.push(aabb3_to_plan(&part.bounds, PlanAxes::XZ));
			commit_door_clear(clearances, part.door_clear, PASSAGE_APPROACH_PAD);
			packed.closets.push(part);
		}
		Placed::WalkIn(part) => {
			clearances.push(aabb3_to_plan(&part.bounds, PlanAxes::XZ));
			commit_door_clear(clearances, part.door_clear, PASSAGE_APPROACH_PAD);
			packed.walk_in_closets.push(part);
		}
		Placed::Ensuite(part) => {
			clearances.push(aabb3_to_plan(&part.bounds, PlanAxes::XZ));
			commit_door_clear(clearances, part.door_clear, PASSAGE_APPROACH_PAD);
			packed.ensuites.push(part);
		}
	}
}

fn partition_from_enclosed(host3: &Aabb3d, enclosed: EnclosedRoom) -> BedroomPartition {
	BedroomPartition {
		bounds: plan_to_aabb3(host3, enclosed.room, PlanAxes::XZ),
		walls: enclosed.walls,
		door_id: enclosed.door_id,
		door: enclosed.door,
		door_clear: enclosed.door_clear,
	}
}

fn partition_pack_clearances(clearances: &[Aabb2d], packed: &CommonBedroomPacked) -> Vec<Aabb2d> {
	let mut out = clearances.to_vec();
	for part in packed
		.closets
		.iter()
		.chain(packed.walk_in_closets.iter())
		.chain(packed.ensuites.iter())
	{
		let plan = aabb3_to_plan(&part.bounds, PlanAxes::XZ);
		out.push(inflate_aabb2(plan, PARTITION_SEP));
	}
	out
}

fn storage_pack_clearances(clearances: &[Aabb2d], packed: &CommonBedroomPacked) -> Vec<Aabb2d> {
	let mut out = clearances.to_vec();
	for solid in packed.wardrobes.iter().chain(packed.dressers.iter()) {
		out.push(inflate_aabb2(aabb3_to_plan(solid, PlanAxes::XZ), STORAGE_SEP));
	}
	out
}

fn xz_area(a: &Aabb3d) -> f32 {
	let e = a.max - a.min;
	e.x.max(0.0) * e.z.max(0.0)
}

fn base_bed_extent(spaciousness: f32) -> Vec3 {
	Vec3::new(2.0, 0.55, 1.6) * Vec3::new(spaciousness, 1.0, spaciousness)
}

fn base_nightstand_extent(spaciousness: f32) -> Vec3 {
	let s = 0.45 * spaciousness;
	Vec3::new(s, 0.5 * spaciousness.min(1.2), s)
}

fn base_closet_depth(spaciousness: f32) -> f32 {
	(0.75 * spaciousness).clamp(0.45, 2.0)
}

fn base_closet_length(spaciousness: f32) -> f32 {
	(1.6 * spaciousness).clamp(0.9, 4.0)
}

fn base_walk_in_depth(spaciousness: f32) -> f32 {
	(1.5 * spaciousness).clamp(1.2, 3.0)
}

fn base_walk_in_length(spaciousness: f32) -> f32 {
	(2.4 * spaciousness).clamp(2.0, 5.0)
}

fn base_ensuite_depth(spaciousness: f32) -> f32 {
	(1.8 * spaciousness).clamp(1.5, 3.2)
}

fn base_ensuite_length(spaciousness: f32) -> f32 {
	(2.6 * spaciousness).clamp(2.2, 5.5)
}

fn collides_clearances(candidate: Aabb2d, clearances: &[Aabb2d]) -> bool {
	clearances.iter().any(|c| intersects_aabb2(candidate, *c))
}

fn collides_solids(candidate: &Aabb3d, packed: &CommonBedroomPacked) -> bool {
	packed
		.beds
		.iter()
		.chain(packed.nightstands.iter())
		.chain(packed.small_bedroom_furniture.iter())
		.chain(packed.wardrobes.iter())
		.chain(packed.dressers.iter())
		.chain(packed.bedroom_furniture.iter())
		.chain(packed.closets.iter().map(|p| &p.bounds))
		.chain(packed.walk_in_closets.iter().map(|p| &p.bounds))
		.chain(packed.ensuites.iter().map(|p| &p.bounds))
		.any(|a| aabb3_intersects(candidate, a))
}

fn aabb3_intersects(a: &Aabb3d, b: &Aabb3d) -> bool {
	a.min.x < b.max.x - 1e-4
		&& a.max.x > b.min.x + 1e-4
		&& a.min.y < b.max.y - 1e-4
		&& a.max.y > b.min.y + 1e-4
		&& a.min.z < b.max.z - 1e-4
		&& a.max.z > b.min.z + 1e-4
}

fn fits(
	candidate: &Aabb3d,
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
) -> bool {
	const EPS: f32 = 1e-3;
	if candidate.min.x < host3.min.x - EPS
		|| candidate.min.y < host3.min.y - EPS
		|| candidate.min.z < host3.min.z - EPS
		|| candidate.max.x > host3.max.x + EPS
		|| candidate.max.y > host3.max.y + EPS
		|| candidate.max.z > host3.max.z + EPS
	{
		return false;
	}
	let plan = aabb3_to_plan(candidate, PlanAxes::XZ);
	if plan.min.x < host.min.x - EPS
		|| plan.min.y < host.min.y - EPS
		|| plan.max.x > host.max.x + EPS
		|| plan.max.y > host.max.y + EPS
	{
		return false;
	}
	if collides_clearances(plan, clearances) {
		return false;
	}
	!collides_solids(candidate, packed)
}

fn place_bed(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
	against_wall: bool,
) -> Option<Aabb3d> {
	let extent = base_bed_extent(spaciousness);
	let size = host3.max - host3.min;
	if extent.x > size.x + 1e-3 || extent.z > size.z + 1e-3 {
		return None;
	}
	if against_wall {
		if let Some(bed) =
			place_bed_against_wall(host3, host, clearances, packed, noise, salt, extent)
		{
			return Some(bed);
		}
	}
	place_bed_free(host3, host, clearances, packed, noise, salt, extent)
}

fn place_bed_against_wall(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	let max_u = (size.x - extent.x).max(0.0);
	let max_v = (size.z - extent.z).max(0.0);
	let start = (noise.sample_unit_4d(salt as f32, 0.0, 0.0, 22.0) * 4.0).floor() as u32 % 4;
	for k in 0..4u32 {
		let wall = (start + k) % 4;
		for attempt in 0..8u32 {
			let t = noise.sample_unit_4d(salt as f32, attempt as f32, wall as f32, 23.0);
			let min = match wall {
				0 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.min.z),
				1 => Vec3::new(host3.min.x + t * max_u, host3.min.y, host3.max.z - extent.z),
				2 => Vec3::new(host3.min.x, host3.min.y, host3.min.z + t * max_v),
				_ => Vec3::new(host3.max.x - extent.x, host3.min.y, host3.min.z + t * max_v),
			};
			let candidate = Aabb3d::from_min_max(min, min + extent);
			if fits(&candidate, host3, host, clearances, packed)
				&& abuts_host_wall(aabb3_to_plan(&candidate, PlanAxes::XZ), host)
			{
				return Some(candidate);
			}
		}
	}
	None
}

fn place_bed_free(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
) -> Option<Aabb3d> {
	let size = host3.max - host3.min;
	for attempt in 0..12u32 {
		let u = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 20.0);
		let v = noise.sample_unit_4d(salt as f32, attempt as f32, 0.0, 21.0);
		let max_u = (size.x - extent.x).max(0.0);
		let max_v = (size.z - extent.z).max(0.0);
		let min = Vec3::new(host3.min.x + u * max_u, host3.min.y, host3.min.z + v * max_v);
		let candidate = Aabb3d::from_min_max(min, min + extent);
		if fits(&candidate, host3, host, clearances, packed) {
			return Some(candidate);
		}
	}
	None
}

fn abuts_host_wall(plan: Aabb2d, host: Aabb2d) -> bool {
	const EPS: f32 = 0.06;
	(plan.min.x - host.min.x).abs() < EPS
		|| (plan.max.x - host.max.x).abs() < EPS
		|| (plan.min.y - host.min.y).abs() < EPS
		|| (plan.max.y - host.max.y).abs() < EPS
}

fn place_small_box(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	spaciousness: f32,
) -> Option<Placed> {
	let extent = base_nightstand_extent(spaciousness);
	let gap = 0.08_f32 * spaciousness;
	if let Some(ns) =
		place_adjacent_to_bed(host3, host, clearances, packed, noise, salt, extent, gap)
	{
		return Some(Placed::Nightstand(ns));
	}
	try_free_extent(
		host3,
		host,
		clearances,
		noise,
		salt,
		FreeExtentKnobs { extent, prefer_wall: false, wall_eps: WALL_EPS, attempts: 10 },
	)
	.filter(|c| !collides_solids(c, packed))
	.map(Placed::SmallFurniture)
}

fn place_adjacent_to_bed(
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	packed: &CommonBedroomPacked,
	noise: &NoiseConfig,
	salt: u32,
	extent: Vec3,
	gap: f32,
) -> Option<Aabb3d> {
	for (bi, bed) in packed.beds.iter().enumerate() {
		let bed_w = bed.max.x - bed.min.x;
		let bed_d = bed.max.z - bed.min.z;
		let long_sides: [u32; 2] = if bed_w + 1e-3 >= bed_d { [2, 3] } else { [0, 1] };
		let start =
			(noise.sample_unit_4d(salt as f32, bi as f32, 0.0, 30.0) * 2.0).floor() as usize % 2;
		for k in 0..2usize {
			let side = long_sides[(start + k) % 2];
			let mid_x = bed.min.x + bed_w * 0.5 - extent.x * 0.5;
			let mid_z = bed.min.z + bed_d * 0.5 - extent.z * 0.5;
			let min = match side {
				0 => Vec3::new(bed.max.x + gap, host3.min.y, mid_z),
				1 => Vec3::new(bed.min.x - gap - extent.x, host3.min.y, mid_z),
				2 => Vec3::new(mid_x, host3.min.y, bed.max.z + gap),
				_ => Vec3::new(mid_x, host3.min.y, bed.min.z - gap - extent.z),
			};
			let candidate = Aabb3d::from_min_max(min, min + extent);
			if fits(&candidate, host3, host, clearances, packed) && abuts_bed(&candidate, bed, gap)
			{
				return Some(candidate);
			}
		}
	}
	None
}

fn abuts_bed(candidate: &Aabb3d, bed: &Aabb3d, gap: f32) -> bool {
	let c = aabb3_to_plan(candidate, PlanAxes::XZ);
	let b = aabb3_to_plan(bed, PlanAxes::XZ);
	touches_aabb2(c, inflate_aabb2(b, gap + 0.05)) && !intersects_aabb2(c, b)
}
