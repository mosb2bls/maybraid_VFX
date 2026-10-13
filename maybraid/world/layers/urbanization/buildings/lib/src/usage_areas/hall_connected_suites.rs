//! Hall carve → residual groups → hall door + enclosure walls.
//!
//! Geometric pipeline shared by residential (and potentially other) fills:
//! 1. [`HallConnectedGroups`] — [`HallsToShafts`] → split → pack to targets
//! 2. [`HallEnclosedSuites`] — one hall door / group + partition / hall-edge walls
//!
//! Program fill (e.g. [`super::livable_apartment::LivableApartment`]) is left to
//! the caller.

use std::collections::HashMap;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;

use crate::fit::{aabb_xz_extent, Confines, FillRegion, FitError, MultiConfines, SpaceKind};
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::shells::ortho::WallEdge;
use crate::usage_areas::boundary_openings::{host_face_spans, plan_edge_excluded};
use crate::usage_areas::halls_to_shafts::{HallsToShafts, HallsToShaftsOptions};
use crate::usage_areas::plan_access::PlanAccessParams;
use crate::usage_areas::plan_cells::{
	cell_has_hall_frontage, cells_edge_adjacent, pack_apartments_to_targets, shared_edge_span,
	split_oversized_cells, split_toward_min_room, PlanCell, MIN_GROUP_CONNECTIVITY,
};
use crate::usage_areas::plan_geom::host_xz;

const EPS: f32 = 1e-3;
const DEFAULT_DOOR_WIDTH: f32 = 1.1 * crate::usage_areas::plan_access::DOOR_SIZE_SCALE;
const DEFAULT_MIN_ROOM: f32 = 2.5;

/// Knobs for packing hall-connected residual groups.
#[derive(Debug, Clone, PartialEq)]
pub struct HallSuitePackParams {
	pub hall_width: Option<f32>,
	/// Target suite areas in m² (catalog order; large → small preferred).
	pub targets: Vec<f32>,
	pub min_room: f32,
	pub min_connectivity: f32,
}

impl Default for HallSuitePackParams {
	fn default() -> Self {
		Self {
			hall_width: None,
			targets: Vec::new(),
			min_room: DEFAULT_MIN_ROOM,
			min_connectivity: MIN_GROUP_CONNECTIVITY,
		}
	}
}

impl HallSuitePackParams {
	/// Access metrics for split / pack (override room + join from these knobs).
	pub fn access(&self) -> PlanAccessParams {
		PlanAccessParams::residential()
			.with_room_min(self.min_room)
			.with_group_connect(self.min_connectivity)
	}
}

/// Knobs for enclosing packed groups.
#[derive(Debug, Clone, PartialEq)]
pub struct HallSuiteEncloseParams {
	/// Opening-id scope (caller module).
	pub scope: &'static str,
	pub door_width: f32,
}

impl Default for HallSuiteEncloseParams {
	fn default() -> Self {
		Self { scope: "hall_connected_suites", door_width: DEFAULT_DOOR_WIDTH }
	}
}

/// Layer ①: halls + residual cells packed into hall-connected groups.
#[derive(Debug, Clone, PartialEq)]
pub struct HallConnectedGroups {
	pub confines: Confines,
	pub halls: HallsToShafts,
	pub cells: Vec<PlanCell>,
	/// Cell-id groups (each touches a hall via ≥1 member).
	pub groups: Vec<Vec<u32>>,
	/// Non-room residuals from HTS (hallways, shafts, …).
	pub residual_within: Vec<FillRegion>,
	pub hall_width: f32,
	pub y0: f32,
	pub y1: f32,
	pub roll: f32,
}

impl HallConnectedGroups {
	pub fn from_confines(
		confines: &Confines,
		noise: NoiseParams,
		params: HallSuitePackParams,
	) -> Result<Self, FitError> {
		let min_room = params.min_room.max(EPS);
		let fp = aabb_xz_extent(&confines.bounds);
		if fp.x + EPS < min_room || fp.y + EPS < min_room {
			return Err(FitError::TooSmall { reason: "hall_suites_host" });
		}
		if params.targets.is_empty() {
			return Err(FitError::InvalidConfines { reason: "hall_suites_empty_targets" });
		}

		let (halls, hts_regions) = HallsToShafts::from_confines_with(
			confines,
			noise,
			HallsToShaftsOptions { hall_width: params.hall_width },
		)?;
		let hall_width = halls.hall_width;
		let hall_bands = halls.hall_bands.clone();

		let y0 = Vec3::from(confines.bounds.min).y;
		let y1 = Vec3::from(confines.bounds.max).y;
		let roll = confines.roll;

		let mut residual_within = Vec::new();
		let mut seed_cells = Vec::new();
		let mut next_id = 0u32;

		for region in hts_regions.within {
			match region.kind {
				SpaceKind::Hallway => residual_within.push(region),
				SpaceKind::InternalSpace => {
					let xz = host_xz(&region.confines.bounds);
					seed_cells.push(PlanCell::new(next_id, xz));
					next_id = next_id.saturating_add(1);
				}
				_ => residual_within.push(region),
			}
		}

		if seed_cells.is_empty() {
			return Ok(Self {
				confines: confines.clone(),
				halls,
				cells: Vec::new(),
				groups: Vec::new(),
				residual_within,
				hall_width,
				y0,
				y1,
				roll,
			});
		}

		let min_room_v = Vec2::splat(min_room);
		let mut cells = split_toward_min_room(&seed_cells, min_room_v, &mut next_id);
		let min_target = params.targets.iter().copied().fold(f32::INFINITY, f32::min).max(12.0);
		let max_cell_area = (min_target * 0.55).max(min_room * min_room * 2.0);
		cells = split_oversized_cells(&cells, max_cell_area, min_room_v, &mut next_id);

		let mut groups =
			pack_apartments_to_targets(&cells, &hall_bands, &params.targets, params.access());
		if groups.is_empty() {
			groups = cells.iter().map(|c| vec![c.id]).collect();
		}

		Ok(Self {
			confines: confines.clone(),
			halls,
			cells,
			groups,
			residual_within,
			hall_width,
			y0,
			y1,
			roll,
		})
	}
}

/// Layer ②: enclosed suites ready for program fill.
#[derive(Debug, Clone, PartialEq)]
pub struct HallEnclosedSuites {
	pub confines: Confines,
	pub halls: HallsToShafts,
	pub hall_width: f32,
	/// One multi-cell confines per group (hall door on the frontage cell).
	pub suites: Vec<MultiConfines>,
	pub walls: Vec<ClippedRectangularStrip>,
	/// Hallways + ungrouped internal scraps.
	pub residual_within: Vec<FillRegion>,
}

impl HallEnclosedSuites {
	pub fn from_groups(packed: HallConnectedGroups, enclose: HallSuiteEncloseParams) -> Self {
		let HallConnectedGroups {
			confines,
			halls,
			cells,
			groups,
			mut residual_within,
			hall_width,
			y0,
			y1,
			roll,
		} = packed;

		if cells.is_empty() {
			return Self {
				confines,
				halls,
				hall_width,
				suites: Vec::new(),
				walls: Vec::new(),
				residual_within,
			};
		}

		let hall_bands = halls.hall_bands.clone();
		let cell_by_id: HashMap<u32, usize> =
			cells.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
		let group_of: HashMap<u32, usize> = groups
			.iter()
			.enumerate()
			.flat_map(|(gi, g)| g.iter().map(move |&id| (id, gi)))
			.collect();

		let mut door_openings = Openings::new();
		let mut cell_openings: HashMap<u32, Openings> = HashMap::new();
		for (gi, group) in groups.iter().enumerate() {
			let Some(door) = group_hall_door(
				group,
				&cells,
				&hall_bands,
				gi as u32,
				y0,
				y1,
				enclose.scope,
				enclose.door_width,
			) else {
				continue;
			};
			door_openings.insert(door.0.clone(), door.1.clone());
			cell_openings.entry(door.2).or_insert_with(Openings::new).insert(door.0, door.1);
		}

		let host = host_xz(&confines.bounds);
		let walls = enclosure_walls(
			&cells,
			&hall_bands,
			&group_of,
			&door_openings,
			&confines.openings,
			host,
			y0,
			y1,
		);

		let mut suites = Vec::new();
		for group in &groups {
			let mut parts = Vec::new();
			for &cid in group {
				let Some(&ci) = cell_by_id.get(&cid) else {
					continue;
				};
				let mut openings = cell_openings.get(&cid).cloned().unwrap_or_default();
				// Inherit host Boundary/Exclusion so room enclosure skips shell faces.
				inherit_host_boundaries(&mut openings, &confines.openings, cells[ci].bounds);
				let bounds = aabb2_to_aabb3(cells[ci].bounds, y0, y1);
				parts.push(FillRegion::new(
					SpaceKind::InternalSpace,
					Confines::new(bounds, roll, openings),
				));
			}
			if !parts.is_empty() {
				suites.push(MultiConfines::new(parts));
			}
		}

		for cell in &cells {
			if group_of.contains_key(&cell.id) {
				continue;
			}
			let openings = cell_openings.get(&cell.id).cloned().unwrap_or_default();
			residual_within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(aabb2_to_aabb3(cell.bounds, y0, y1), roll, openings),
			));
		}

		Self { confines, halls, hall_width, suites, walls, residual_within }
	}

	/// Pack + enclose in one step.
	pub fn from_confines(
		confines: &Confines,
		noise: NoiseParams,
		pack: HallSuitePackParams,
		enclose: HallSuiteEncloseParams,
	) -> Result<Self, FitError> {
		let groups = HallConnectedGroups::from_confines(confines, noise, pack)?;
		Ok(Self::from_groups(groups, enclose))
	}
}

fn aabb2_to_aabb3(a: Aabb2d, y0: f32, y1: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(a.min.x, y0, a.min.y), Vec3::new(a.max.x, y1, a.max.y))
}

/// One hall door for the whole group, authored on the best frontage cell.
///
/// Prefer the **least skinny** frontage cell (largest min extent, then area)
/// over longest hall edge — a merged bowling-alley strip often has a longer
/// hall contact but is a bad door host.
fn group_hall_door(
	group: &[u32],
	cells: &[PlanCell],
	halls: &[Aabb2d],
	group_id: u32,
	y0: f32,
	y1: f32,
	scope: &str,
	door_width: f32,
) -> Option<(OpeningId, Opening, u32)> {
	// (cid, along_x, lo, hi, mid, min_ext, area, shared_len)
	let mut best: Option<(u32, bool, f32, f32, f32, f32, f32, f32)> = None;
	for &cid in group {
		let Some(cell) = cells.iter().find(|c| c.id == cid) else {
			continue;
		};
		if !cell_has_hall_frontage(cell, halls, MIN_GROUP_CONNECTIVITY, EPS) {
			continue;
		}
		let area = cell.area();
		let size = cell.size();
		let min_ext = size.x.min(size.y);
		for hall in halls {
			if let Some(span) = shared_edge_span(cell.bounds, *hall) {
				let len = span.2 - span.1;
				if len + EPS < door_width {
					continue;
				}
				let better = match best {
					None => true,
					Some((_, _, _, _, _, be, ba, bl)) => {
						min_ext > be + EPS
							|| ((min_ext - be).abs() <= EPS && area > ba + EPS)
							|| ((min_ext - be).abs() <= EPS && (area - ba).abs() <= EPS && len > bl)
					}
				};
				if better {
					best = Some((cid, span.0, span.1, span.2, span.3, min_ext, area, len));
				}
			}
		}
	}
	let (cell_id, along_x, lo, hi, mid, _min_ext, _area, shared_len) = best?;
	if shared_len < door_width + EPS {
		return None;
	}
	let clear = door_width.min(shared_len - 0.2);
	let center = 0.5 * (lo + hi);
	let half = clear * 0.5;
	let door_lo = (center - half).max(lo);
	let door_hi = (center + half).min(hi);
	let half_d = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.06).max(0.12);
	let door_h = crate::usage_areas::plan_access::door_leaf_height(y1 - y0);
	let bounds = if along_x {
		Aabb3d::from_min_max(
			Vec3::new(door_lo, y0, mid - half_d),
			Vec3::new(door_hi, y0 + door_h, mid + half_d),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(mid - half_d, y0, door_lo),
			Vec3::new(mid + half_d, y0 + door_h, door_hi),
		)
	};
	Some((
		OpeningId::scoped(scope, "hall_door", group_id.to_string()),
		Opening::new(bounds, OpeningLabel::Passage),
		cell_id,
	))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct WallLineKey {
	along_x: bool,
	mid_mm: i32,
}

#[derive(Debug, Clone, Copy)]
struct WallSpan {
	lo: f32,
	hi: f32,
	outward: Vec2,
}

fn inherit_host_boundaries(dst: &mut Openings, host: &Openings, cell: Aabb2d) {
	for (id, o) in host.iter() {
		if !matches!(o.label, OpeningLabel::Boundary | OpeningLabel::Exclusion) {
			continue;
		}
		let omin = Vec3::from(o.bounds.min);
		let omax = Vec3::from(o.bounds.max);
		let ox = Aabb2d { min: Vec2::new(omin.x, omin.z), max: Vec2::new(omax.x, omax.z) };
		// Keep if the opening overlaps this cell's inflated footprint.
		let infl = Aabb2d { min: cell.min - Vec2::splat(0.2), max: cell.max + Vec2::splat(0.2) };
		if ox.max.x < infl.min.x
			|| ox.min.x > infl.max.x
			|| ox.max.y < infl.min.y
			|| ox.min.y > infl.max.y
		{
			continue;
		}
		dst.insert(id.clone(), o.clone());
	}
}

fn enclosure_walls(
	cells: &[PlanCell],
	halls: &[Aabb2d],
	group_of: &HashMap<u32, usize>,
	door_openings: &Openings,
	host_openings: &Openings,
	host: Aabb2d,
	y0: f32,
	y1: f32,
) -> Vec<ClippedRectangularStrip> {
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	let height = (y1 - y0).max(2.0);
	let mut pending: HashMap<WallLineKey, Vec<WallSpan>> = HashMap::new();

	let push_span = |pending: &mut HashMap<WallLineKey, Vec<WallSpan>>,
	                 along_x: bool,
	                 lo: f32,
	                 hi: f32,
	                 mid: f32,
	                 outward: Vec2| {
		let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
		if hi - lo < EPS {
			return;
		}
		let key = WallLineKey { along_x, mid_mm: (mid * 1000.0).round() as i32 };
		pending.entry(key).or_default().push(WallSpan { lo, hi, outward });
	};

	for i in 0..cells.len() {
		for j in (i + 1)..cells.len() {
			let a = &cells[i];
			let b = &cells[j];
			if !cells_edge_adjacent(a, b, EPS) {
				continue;
			}
			let ga = group_of.get(&a.id).copied();
			let gb = group_of.get(&b.id).copied();
			let seal = match (ga, gb) {
				(Some(x), Some(y)) => x != y,
				(Some(_), None) | (None, Some(_)) => true,
				_ => false,
			};
			if !seal {
				continue;
			}
			let Some((along_x, lo, hi, mid)) = shared_edge_span(a.bounds, b.bounds) else {
				continue;
			};
			let toward = b.center();
			let from = Vec2::new(
				if along_x { 0.5 * (lo + hi) } else { mid },
				if along_x { mid } else { 0.5 * (lo + hi) },
			);
			push_span(&mut pending, along_x, lo, hi, mid, outward_toward(from, toward, along_x));
		}
	}

	for cell in cells {
		if !group_of.contains_key(&cell.id) {
			continue;
		}
		for hall in halls {
			let Some((along_x, lo, hi, mid)) = shared_edge_span(cell.bounds, *hall) else {
				continue;
			};
			// Skip hall-kiss nubs — same threshold as suite frontage. Tiny spans
			// next to a real door read as pinches at the entry.
			if hi - lo + EPS < MIN_GROUP_CONNECTIVITY {
				continue;
			}
			let from = Vec2::new(
				if along_x { 0.5 * (lo + hi) } else { mid },
				if along_x { mid } else { 0.5 * (lo + hi) },
			);
			let toward = 0.5 * (hall.min + hall.max);
			push_span(&mut pending, along_x, lo, hi, mid, outward_toward(from, toward, along_x));
		}

		// Host-perimeter boxing: wall faces on the primary-rect boundary unless
		// Boundary/Exclusion (exterior shell or progressive sibling handoff).
		for (along_x, mid, flo, fhi) in host_face_spans(host) {
			let on_host = if along_x {
				(cell.bounds.min.y - mid).abs() < 0.08 || (cell.bounds.max.y - mid).abs() < 0.08
			} else {
				(cell.bounds.min.x - mid).abs() < 0.08 || (cell.bounds.max.x - mid).abs() < 0.08
			};
			if !on_host {
				continue;
			}
			let (lo, hi) = if along_x {
				(cell.bounds.min.x.max(flo), cell.bounds.max.x.min(fhi))
			} else {
				(cell.bounds.min.y.max(flo), cell.bounds.max.y.min(fhi))
			};
			if hi - lo < EPS {
				continue;
			}
			if plan_edge_excluded(host_openings, along_x, lo, hi, mid) {
				continue;
			}
			// Skip spans already sealed as hall frontage.
			let on_hall = halls.iter().any(|h| {
				shared_edge_span(cell.bounds, *h).is_some_and(|(ax, a, b, m)| {
					ax == along_x && (m - mid).abs() < 0.08 && b > lo + EPS && a < hi - EPS
				})
			});
			if on_hall {
				continue;
			}
			let from = Vec2::new(
				if along_x { 0.5 * (lo + hi) } else { mid },
				if along_x { mid } else { 0.5 * (lo + hi) },
			);
			let toward_out = if along_x {
				Vec2::new(from.x, mid + if (mid - host.min.y).abs() < 0.08 { -1.0 } else { 1.0 })
			} else {
				Vec2::new(mid + if (mid - host.min.x).abs() < 0.08 { -1.0 } else { 1.0 }, from.y)
			};
			push_span(
				&mut pending,
				along_x,
				lo,
				hi,
				mid,
				outward_toward(from, toward_out, along_x),
			);
		}
	}

	// Hall doors + host passages (e.g. inter-rect) cut voids in enclosure walls.
	let mut cut_openings = door_openings.clone();
	for (id, o) in host_openings.iter() {
		if matches!(o.label, OpeningLabel::Passage) {
			cut_openings.insert(id.clone(), o.clone());
		}
	}

	let mut walls = Vec::new();
	for (line, spans) in pending {
		let mid = line.mid_mm as f32 / 1000.0;
		for span in merge_wall_spans(spans) {
			if let Some(wall) = author_wall_span(
				line.along_x,
				span.lo,
				span.hi,
				mid,
				span.outward,
				&cut_openings,
				y0,
				height,
				thickness,
			) {
				walls.push(wall);
			}
		}
	}
	walls
}

fn merge_wall_spans(mut spans: Vec<WallSpan>) -> Vec<WallSpan> {
	if spans.is_empty() {
		return spans;
	}
	spans.sort_by(|a, b| a.lo.partial_cmp(&b.lo).unwrap_or(std::cmp::Ordering::Equal));
	let mut out = Vec::new();
	let mut cur = spans[0];
	for span in spans.into_iter().skip(1) {
		if span.lo <= cur.hi + EPS {
			let cur_len = cur.hi - cur.lo;
			let span_len = span.hi - span.lo;
			cur.hi = cur.hi.max(span.hi);
			if span_len > cur_len {
				cur.outward = span.outward;
			}
		} else {
			out.push(cur);
			cur = span;
		}
	}
	out.push(cur);
	out
}

fn outward_toward(from: Vec2, toward: Vec2, along_x: bool) -> Vec2 {
	let d = toward - from;
	if along_x {
		if d.y >= 0.0 {
			Vec2::Y
		} else {
			-Vec2::Y
		}
	} else if d.x >= 0.0 {
		Vec2::X
	} else {
		-Vec2::X
	}
}

fn author_wall_span(
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	outward: Vec2,
	openings: &Openings,
	y0: f32,
	height: f32,
	thickness: f32,
) -> Option<ClippedRectangularStrip> {
	if (hi - lo).abs() < EPS {
		return None;
	}
	let (start, end) = if along_x {
		(Vec3::new(lo, y0, mid), Vec3::new(hi, y0, mid))
	} else {
		(Vec3::new(mid, y0, lo), Vec3::new(mid, y0, hi))
	};
	let edge = WallEdge::new(start, end, height, outward.normalize_or_zero());
	Some(ClippedRectangularStrip::from_wall_edge_with_passage_openings(edge, openings, thickness))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::usage_areas::plan_cells::PlanCell;
	use bevy_math::Vec2;

	#[test]
	fn hall_door_prefers_fatter_frontage_cell() {
		// Fat seed + thin long strip both touch the hall. Door must land on the
		// fat cell even though the strip has a longer hall edge.
		let cells = vec![
			PlanCell::new(0, Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(6.0, 5.0) }),
			PlanCell::new(1, Aabb2d { min: Vec2::new(6.0, 0.0), max: Vec2::new(8.0, 9.0) }),
		];
		let halls = [Aabb2d { min: Vec2::new(0.0, -2.0), max: Vec2::new(8.0, 0.0) }];
		let door =
			group_hall_door(&[0, 1], &cells, &halls, 0, 0.0, 3.0, "test", 1.0).expect("door");
		assert_eq!(door.2, 0, "door should prefer deep cell 0, got cell {}", door.2);
	}
}
