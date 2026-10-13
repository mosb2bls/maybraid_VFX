//! Apartment layout: orchestrate reusable plan helpers into one residential fill.
//!
//! # Pipeline
//!
//! ```text
//! MultiConfines (suite cells)
//!       │
//!       ▼
//! 1. validate envelope          — walkable / room-capable / height gates
//!       │
//!       ▼
//! 2. carve entry                — [`partition_entry_and_body`] → Entryway bands
//!       │                         + body rects + scraps
//!       ▼
//! 3. max-rect passage cluster   — [`RectPassageCluster`] on body (tree doors)
//!       │
//!       ▼
//! 4. program slice → RLA        — one [`RectangularLivableArea`] per max-rect
//!       │                         with entry bands as *circulation anchors*
//!       ▼
//! 5. reclaim uncovered scraps   — closet band or InternalSpace residual
//!       │
//!       ▼
//! 6. soft normalize             — demote closed rooms that cannot touch open
//!                                 circulation (closet-sized vs OpenHall)
//! ```
//!
//! Most stages call shared usage-area primitives (`plan_access`, `plan_cells`,
//! `rect_passage_cluster`, `rectangular_livable_area`). What follows is **bespoke
//! to apartments** — keep these decisions in mind when designing similar
//! multi-cell residential (or hotel / dorm) fills.
//!
//! # Bespoke decisions (why not a generic “suite fill”)
//!
//! | Choice | Why |
//! |--------|-----|
//! | **Entry carved before body cluster** | Hall-connected suites seed from corridor frontage; apartments seed from a *unit entry door* and need a dedicated stem so the body can ignore that strip. |
//! | **Entry bands as RLA circulation anchors, not max-rect host openings** | Putting entry doors on the max-rect host makes RLA normalize require every closed room to face that door. Open furniture still needs keep-outs on the stem — pass bands into [`RectangularLivableArea::fit_with_circulation`] instead. |
//! | **Program from total m², then distribute across max-rects** | One apartment program (beds / baths / open) split over irregular L/T bodies; commercial stalls usually pick one interior per bay. |
//! | **RLA `TooSmall` → OpenHall + walkway** | Soft failure keeps the envelope claimed as circulation rather than leaving a hole or panicking. Small rects also emit `InternalSpace` residual. |
//! | **Soft normalize with wall-gap adjacency** | SpineHall bedrooms sit ~panel thickness off the hall. Exact edge touch false-demoted every walled closed room to empty OpenHall while leaving partitions. Any open circ / walkway counts — not only an entry-flood component. |
//! | **Closet-size demotion only** | Unreachable closed rooms in `1.8..8` m² become HouseholdCloset; larger demotions reopen as OpenHall so normalize cannot flood oversized “closets”. |
//! | **EatingArea → Kitchen + optional Dining** | RLA packs `Eating` as one quarter; the apartment room list exposes kitchen/dining as separate [`ApartmentRoom`] variants for IR / labels. |
//!
//! Shared pieces that *should* be reused elsewhere: [`PlanAccessParams::residential`],
//! [`RectPassageCluster`], [`RectangularLivableArea`], scrap → closet banding in
//! [`push_leftover`].

use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use building_components::labels::{LabelNode, LabelStyle};
use procedural_common::{aabb2_area, inflate_aabb2, NoiseParams};

use crate::fit::{Confines, FillRegion, FillableRegions, FitError, MultiConfines, SpaceKind};
use crate::openings::Openings;
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::plan_access::PlanAccessParams;
use crate::usage_areas::plan_cells::{shared_edge_span, subtract_aabb2};
use crate::usage_areas::plan_geom::{confines_from_xz, host_xz, noise_for_cell};
use crate::usage_areas::rect_passage_cluster::{RectPassageCluster, RectPassageClusterParams};
use crate::usage_areas::rectangular_livable_area::{
	RectAreaRoom, RectLivableStrategy, RectQuarterKind, RectangularLivableArea,
	RectangularLivableAreaParameterized, DEFAULT_CLOSED_MAX_AREA,
};

use super::entry::{collect_work_rects, find_entry_door, partition_entry_and_body, push_entryway};
use super::program::{distribute_program, full_kind_list, program_from_area};
use super::room::ApartmentRoom;
use super::{LivableApartment, EPS, SCOPE};

/// Minimum clear height (m) for a livable cell.
const MIN_CEILING: f32 = 2.0;
/// Max-rect area floor (m²) for the body cluster.
const MIN_MAX_RECT_AREA: f32 = 8.0;
/// Household-closet area band (m²) — scraps and normalize demotions.
const CLOSET_AREA_MIN: f32 = 1.8;
const CLOSET_AREA_MAX: f32 = 8.0;
/// Inflate used when closed rooms sit behind partition thickness off a hall.
const PARTITION_GAP: f32 = DEFAULT_PANEL_THICKNESS + 0.2;

pub(crate) fn residential_access() -> PlanAccessParams {
	PlanAccessParams::residential()
}

/// Fill one apartment from its envelope cells.
///
/// See the module docs for stage order and apartment-specific policy.
pub(crate) fn fit_from_multi(
	region_id: u32,
	cells: &MultiConfines,
	noise: NoiseParams,
) -> Result<(LivableApartment, FillableRegions), FitError> {
	let access = residential_access();
	validate_envelope(cells, access)?;

	let y0 = Vec3::from(cells.parts[0].confines.bounds.min).y;
	let y1 = Vec3::from(cells.parts[0].confines.bounds.max).y;
	let roll = cells.parts[0].confines.roll;
	let apt_noise = noise_for_cell(noise, region_id as i32);
	let program =
		program_from_area(total_footprint_area(cells), apt_noise, cells.parts[0].confines.center());

	let mut rooms = Vec::new();
	let mut residual_within = Vec::new();
	let mut walkways = Vec::new();
	let mut partitions = Vec::new();

	// --- Entry carve -------------------------------------------------------
	let (door_ci, work_rects) = collect_work_rects(cells);
	let door_cell = host_xz(&cells.parts[door_ci].confines.bounds);
	let door_opening = find_entry_door(&cells.parts[door_ci].confines.openings)
		.or_else(|| find_entry_door(&cells.parts[0].confines.openings));
	let partitioned = partition_entry_and_body(
		work_rects,
		door_cell,
		door_opening.as_ref().map(|(_, d)| d),
		access,
	);
	for band in &partitioned.entry_bands {
		push_entryway(&mut rooms, &mut walkways, *band, y0, y1, roll);
	}
	for scrap in partitioned.scraps {
		push_leftover(
			&mut rooms,
			&mut residual_within,
			confines_from_xz(scrap, y0, y1, roll, &Openings::new()),
		);
	}

	let entry_xz = partitioned.entry_bands.first().copied();
	let body = partitioned.body;
	if body.is_empty() {
		return early_entry_only(region_id, cells, rooms, walkways, residual_within);
	}

	// --- Body: cluster → program slices → RLA per max-rect -----------------
	let cluster = RectPassageCluster::from_parts(
		&body,
		entry_xz,
		y0,
		y1,
		roll,
		region_id,
		RectPassageClusterParams {
			min_room: access.room_min,
			min_rect_area: MIN_MAX_RECT_AREA,
			min_access: access.walk_clear,
			scope: SCOPE,
		},
	)
	.ok_or(FitError::TooSmall { reason: "livable_no_rects" })?;

	let kind_list = full_kind_list(program);
	let slices = distribute_program(&kind_list, &cluster.rects, apt_noise);
	let rla_params = RectangularLivableAreaParameterized {
		strategy: RectLivableStrategy::CaseAttempt,
		min_hall: access.walk_clear,
		closed_max_area: DEFAULT_CLOSED_MAX_AREA,
	};

	fit_rla_per_max_rect(
		&cluster,
		&slices,
		rla_params,
		entry_xz,
		&partitioned.entry_bands,
		apt_noise,
		region_id,
		roll,
		&mut rooms,
		&mut walkways,
		&mut partitions,
		&mut residual_within,
	)?;

	reclaim_uncovered_body_scraps(
		&body,
		&cluster.rects,
		y0,
		y1,
		roll,
		&mut rooms,
		&mut residual_within,
	);

	if rooms.is_empty() {
		return Err(FitError::TooSmall { reason: "livable_no_quarters" });
	}

	normalize_apartment_circulation(&mut rooms, entry_xz, access, &walkways, y0, y1, roll);

	Ok((
		LivableApartment {
			region_id,
			cells: cells.clone(),
			rooms,
			walkways,
			partitions,
			max_rects: cluster.rects,
			shell: None,
		},
		FillableRegions { within: residual_within, atop: Vec::new() },
	))
}

// ---------------------------------------------------------------------------
// Stages
// ---------------------------------------------------------------------------

fn validate_envelope(cells: &MultiConfines, access: PlanAccessParams) -> Result<(), FitError> {
	if cells.is_empty() {
		return Err(FitError::TooSmall { reason: "livable_empty" });
	}
	let mut has_body = false;
	for part in cells.iter() {
		let height = (part.confines.bounds.max.y - part.confines.bounds.min.y).max(0.0);
		let xz = host_xz(&part.confines.bounds);
		if !access.is_walkable(xz) {
			return Err(FitError::TooSmall { reason: "livable_footprint" });
		}
		if access.is_room_rect(xz) {
			has_body = true;
		}
		if height < MIN_CEILING {
			return Err(FitError::TooSmall { reason: "livable_height" });
		}
	}
	if !has_body {
		return Err(FitError::TooSmall { reason: "livable_no_body_cell" });
	}
	Ok(())
}

fn total_footprint_area(cells: &MultiConfines) -> f32 {
	cells
		.iter()
		.map(|p| {
			let fp = p.confines.footprint();
			fp.x * fp.y
		})
		.sum()
}

/// Entry-only apartment (stem claimed, no body left to cluster).
fn early_entry_only(
	region_id: u32,
	cells: &MultiConfines,
	rooms: Vec<ApartmentRoom>,
	walkways: Vec<Aabb2d>,
	residual_within: Vec<FillRegion>,
) -> Result<(LivableApartment, FillableRegions), FitError> {
	if rooms.is_empty() {
		return Err(FitError::TooSmall { reason: "livable_no_body" });
	}
	Ok((
		LivableApartment {
			region_id,
			cells: cells.clone(),
			rooms,
			walkways,
			partitions: Vec::new(),
			max_rects: Vec::new(),
			shell: None,
		},
		FillableRegions { within: residual_within, atop: Vec::new() },
	))
}

/// Run RLA on each max-rect; soft-fail to OpenHall.
///
/// **Bespoke:** entry bands are circulation anchors for open-room furniture
/// keep-outs. They are *not* injected as inbound openings on the max-rect host
/// (that breaks RLA normalize for closed rooms).
fn fit_rla_per_max_rect(
	cluster: &RectPassageCluster,
	slices: &[Vec<RectQuarterKind>],
	rla_params: RectangularLivableAreaParameterized,
	entry_xz: Option<Aabb2d>,
	entry_bands: &[Aabb2d],
	apt_noise: NoiseParams,
	region_id: u32,
	roll: f32,
	rooms: &mut Vec<ApartmentRoom>,
	walkways: &mut Vec<Aabb2d>,
	partitions: &mut Vec<ClippedRectangularStrip>,
	residual_within: &mut Vec<FillRegion>,
) -> Result<(), FitError> {
	for (ri, rect) in cluster.rects.iter().enumerate() {
		let confines = cluster.confines_ensured(ri, entry_xz);
		let cell_noise = noise_for_cell(apt_noise, (region_id as i32).wrapping_add(ri as i32 * 17));
		match RectangularLivableArea::fit_with_circulation(
			&confines,
			cell_noise,
			rla_params,
			&slices[ri],
			entry_bands,
		) {
			Ok((rla, nested)) => {
				walkways.extend(rla.walkways.iter().copied());
				partitions.extend(rla.partitions);
				for room in rla.rooms {
					push_mapped_rla_room(rooms, room);
				}
				residual_within.extend(nested.within);
			}
			Err(FitError::TooSmall { .. }) => {
				rooms.push(ApartmentRoom::OpenHall {
					label: label_filling_aabb(LabelStyle::Cyan, "OpenHall", &confines.bounds, roll),
					confines: confines.clone(),
				});
				walkways.push(*rect);
				if aabb2_area(*rect) < MIN_MAX_RECT_AREA {
					residual_within.push(FillRegion::new(SpaceKind::InternalSpace, confines));
				}
			}
			Err(err) => return Err(err),
		}
	}
	Ok(())
}

/// Body area not covered by the max-rect cluster → closet or residual.
fn reclaim_uncovered_body_scraps(
	body: &[Aabb2d],
	max_rects: &[Aabb2d],
	y0: f32,
	y1: f32,
	roll: f32,
	rooms: &mut Vec<ApartmentRoom>,
	residual_within: &mut Vec<FillRegion>,
) {
	let covered: f32 = max_rects.iter().map(|r| aabb2_area(*r)).sum();
	let work: f32 = body.iter().map(|r| aabb2_area(*r)).sum();
	if work <= covered + 1.0 {
		return;
	}
	for scrap in body {
		for s in subtract_aabb2(*scrap, max_rects) {
			if aabb2_area(s) < EPS {
				continue;
			}
			push_leftover(
				rooms,
				residual_within,
				confines_from_xz(s, y0, y1, roll, &Openings::new()),
			);
		}
	}
}

// ---------------------------------------------------------------------------
// Room mapping / plan helpers
// ---------------------------------------------------------------------------

fn push_mapped_rla_room(rooms: &mut Vec<ApartmentRoom>, room: RectAreaRoom) {
	match room {
		RectAreaRoom::OpenBand { label, confines } => {
			rooms.push(ApartmentRoom::OpenHall { label, confines });
		}
		RectAreaRoom::HouseholdCloset { label, confines } => {
			rooms.push(ApartmentRoom::HouseholdCloset { label, confines });
		}
		RectAreaRoom::Bedroom(r) => rooms.push(ApartmentRoom::Bedroom(r)),
		RectAreaRoom::Living(r) => rooms.push(ApartmentRoom::Living(r)),
		RectAreaRoom::Eating(e) => {
			// Bespoke: flatten EatingArea into apartment kitchen + dining rooms.
			rooms.push(ApartmentRoom::Kitchen(e.kitchen));
			if let Some(d) = e.dining {
				rooms.push(ApartmentRoom::Dining(d));
			}
		}
		RectAreaRoom::Kitchen(r) => rooms.push(ApartmentRoom::Kitchen(r)),
		RectAreaRoom::Dining(r) => rooms.push(ApartmentRoom::Dining(r)),
		RectAreaRoom::Bathroom(r) => rooms.push(ApartmentRoom::Bathroom(r)),
		RectAreaRoom::HalfBath(r) => rooms.push(ApartmentRoom::HalfBath(r)),
		RectAreaRoom::Sitting(r) => rooms.push(ApartmentRoom::Sitting(r)),
		RectAreaRoom::Study(r) => rooms.push(ApartmentRoom::Study(r)),
	}
}

pub(crate) fn room_xz(room: &ApartmentRoom) -> Option<Aabb2d> {
	match room {
		ApartmentRoom::Entryway { confines, .. }
		| ApartmentRoom::HouseholdCloset { confines, .. }
		| ApartmentRoom::OpenHall { confines, .. } => Some(host_xz(&confines.bounds)),
		ApartmentRoom::Bedroom(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Living(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Kitchen(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Dining(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Bathroom(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::HalfBath(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Sitting(r) => Some(label_xz(&r.room_type)),
		ApartmentRoom::Study(r) => Some(label_xz(&r.room_type)),
	}
}

fn label_xz(label: &LabelNode) -> Aabb2d {
	let c = label.placement.translation;
	let e = label.placement.scale;
	Aabb2d {
		min: Vec2::new(c.x - e.x * 0.5, c.z - e.z * 0.5),
		max: Vec2::new(c.x + e.x * 0.5, c.z + e.z * 0.5),
	}
}

// ---------------------------------------------------------------------------
// Soft normalize (apartment-specific adjacency policy)
// ---------------------------------------------------------------------------

/// Demote closed rooms that do not abut any open circulation.
///
/// **Bespoke policy:**
/// - Reachability = shared edge (or partition-gap inflate) with *any* open
///   circ / walkway / entry — not BFS from the entry door alone.
/// - Closet-sized isolates → [`ApartmentRoom::HouseholdCloset`]; larger →
///   [`ApartmentRoom::OpenHall`] (avoids oversized-closet floods).
fn normalize_apartment_circulation(
	rooms: &mut Vec<ApartmentRoom>,
	entry: Option<Aabb2d>,
	access: PlanAccessParams,
	walkways: &[Aabb2d],
	y0: f32,
	y1: f32,
	roll: f32,
) {
	let Some(entry) = entry else {
		return;
	};
	let door = access.door_contact();
	let open_rects: Vec<Aabb2d> = rooms
		.iter()
		.filter(|r| r.is_open_circ())
		.filter_map(room_xz)
		.chain(std::iter::once(entry))
		.chain(walkways.iter().copied())
		.collect();
	if open_rects.is_empty() {
		return;
	}

	for room in rooms.iter_mut() {
		if !room.is_closed() {
			continue;
		}
		let Some(cz) = room_xz(room) else {
			continue;
		};
		if open_rects.iter().any(|o| closed_reaches_open(cz, *o, door)) {
			continue;
		}
		let area = aabb2_area(cz);
		let confines = confines_from_xz(cz, y0, y1, roll, &Openings::new());
		*room = if is_closet_area(area) {
			ApartmentRoom::HouseholdCloset {
				label: label_filling_aabb(
					LabelStyle::Gray,
					"HouseholdCloset",
					&confines.bounds,
					roll,
				),
				confines,
			}
		} else {
			ApartmentRoom::OpenHall {
				label: label_filling_aabb(LabelStyle::Cyan, "OpenHall", &confines.bounds, roll),
				confines,
			}
		};
	}
}

/// True when a closed room abuts open circulation, allowing a partition gap.
fn closed_reaches_open(closed: Aabb2d, open: Aabb2d, min_len: f32) -> bool {
	if shared_edge_span(closed, open).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len) {
		return true;
	}
	let grown = inflate_aabb2(closed, PARTITION_GAP);
	shared_edge_span(grown, open).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len)
		|| shared_edge_span(closed, inflate_aabb2(open, PARTITION_GAP))
			.is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= min_len)
}

fn is_closet_area(area: f32) -> bool {
	(CLOSET_AREA_MIN..CLOSET_AREA_MAX).contains(&area)
}

/// Scraps in the closet area band become HouseholdCloset; else InternalSpace.
fn push_leftover(
	rooms: &mut Vec<ApartmentRoom>,
	residual: &mut Vec<FillRegion>,
	confines: Confines,
) {
	let area = {
		let fp = confines.footprint();
		fp.x * fp.y
	};
	if is_closet_area(area) {
		rooms.push(ApartmentRoom::HouseholdCloset {
			label: label_filling_aabb(
				LabelStyle::Gray,
				"HouseholdCloset",
				&confines.bounds,
				confines.roll,
			),
			confines,
		});
	} else {
		residual.push(FillRegion::new(SpaceKind::InternalSpace, confines));
	}
}
