//! Hall-door entry carve and corridor-stem claiming.

use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use building_components::labels::LabelStyle;
use procedural_common::aabb2_area;

use crate::fit::MultiConfines;
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::plan_access::PlanAccessParams;
use crate::usage_areas::plan_cells::{shared_edge_span, subtract_aabb2};
use crate::usage_areas::plan_geom::{aabb2_near_eq, confines_from_xz, host_xz};

use super::room::ApartmentRoom;
use super::EPS;

const ENTRY_DEPTH: f32 = 1.8;
const ENTRY_WIDTH: f32 = 2.2;

pub(crate) struct EntryBodyPartition {
	pub entry_bands: Vec<Aabb2d>,
	pub body: Vec<Aabb2d>,
	pub scraps: Vec<Aabb2d>,
}

pub(crate) fn collect_work_rects(cells: &MultiConfines) -> (usize, Vec<Aabb2d>) {
	let mut door_ci = 0usize;
	for (i, part) in cells.iter().enumerate() {
		if find_entry_door(&part.confines.openings).is_some() {
			door_ci = i;
			break;
		}
	}
	let rects = cells.iter().map(|p| host_xz(&p.confines.bounds)).collect();
	(door_ci, rects)
}

pub(crate) fn find_entry_door(openings: &Openings) -> Option<(OpeningId, Opening)> {
	openings
		.iter()
		.find(|(_, o)| matches!(o.label, OpeningLabel::Passage))
		.map(|(id, o)| (id.clone(), o.clone()))
}

pub(crate) fn push_entryway(
	rooms: &mut Vec<ApartmentRoom>,
	walkways: &mut Vec<Aabb2d>,
	entry: Aabb2d,
	y0: f32,
	y1: f32,
	roll: f32,
) {
	let entry_c = confines_from_xz(entry, y0, y1, roll, &Openings::new());
	rooms.push(ApartmentRoom::Entryway {
		label: label_filling_aabb(LabelStyle::Cyan, "Entryway", &entry_c.bounds, roll),
		confines: entry_c,
	});
	walkways.push(entry);
}

fn touches_access(r: Aabb2d, bands: &[Aabb2d], access: PlanAccessParams) -> bool {
	let touch = access.open_touch();
	bands.iter().any(|b| {
		aabb2_near_eq(r, *b)
			|| shared_edge_span(r, *b).is_some_and(|(_, lo, hi, _)| hi - lo + EPS >= touch)
	})
}

/// Carve door entry, flood-claim corridor stems, split body vs scraps.
pub(crate) fn partition_entry_and_body(
	work: Vec<Aabb2d>,
	door_cell: Aabb2d,
	door: Option<&Opening>,
	access: PlanAccessParams,
) -> EntryBodyPartition {
	let mut entry_bands = Vec::new();
	let mut pending: Vec<Aabb2d> =
		work.into_iter().filter(|r| aabb2_area(*r) > EPS * EPS).collect();

	if let Some(door) = door {
		if let Some((mut entry, rem)) = carve_entryway(door_cell, door, ENTRY_DEPTH, ENTRY_WIDTH) {
			let usable_rem: Vec<Aabb2d> =
				rem.into_iter().filter(|r| access.is_room_rect(*r)).collect();
			if usable_rem.is_empty() {
				entry = door_cell;
			}
			entry_bands.push(entry);
			pending.retain(|r| !aabb2_near_eq(*r, door_cell));
			pending.extend(usable_rem);
		}
	}

	let mut body = Vec::new();
	let mut rest = Vec::new();
	for r in pending {
		if access.is_room_rect(r) {
			body.push(r);
		} else {
			rest.push(r);
		}
	}

	let mut guard = 0;
	while guard < rest.len().saturating_mul(2).max(4) {
		guard += 1;
		let Some(idx) = rest.iter().position(|r| {
			access.is_access_corridor(*r)
				&& (touches_access(*r, &entry_bands, access) || touches_access(*r, &body, access))
		}) else {
			break;
		};
		entry_bands.push(rest.remove(idx));
	}

	EntryBodyPartition { entry_bands, body, scraps: rest }
}

fn carve_entryway(
	host: Aabb2d,
	door: &Opening,
	depth: f32,
	width: f32,
) -> Option<(Aabb2d, Vec<Aabb2d>)> {
	let dmin = Vec3::from(door.bounds.min);
	let dmax = Vec3::from(door.bounds.max);
	let dc = Vec2::new(0.5 * (dmin.x + dmax.x), 0.5 * (dmin.z + dmax.z));
	let depth = depth.clamp(1.2, 2.8);
	let width = width
		.clamp(1.4, host.max.x - host.min.x - EPS)
		.min(host.max.y - host.min.y - EPS);

	let dist_w = (dc.x - host.min.x).abs();
	let dist_e = (dc.x - host.max.x).abs();
	let dist_s = (dc.y - host.min.y).abs();
	let dist_n = (dc.y - host.max.y).abs();
	let edge = [dist_w, dist_e, dist_s, dist_n]
		.iter()
		.enumerate()
		.min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
		.map(|(i, _)| i)?;

	let half_w = width * 0.5;
	let entry = match edge {
		0 => {
			let y0 = (dc.y - half_w).clamp(host.min.y, host.max.y - width);
			Aabb2d {
				min: Vec2::new(host.min.x, y0),
				max: Vec2::new((host.min.x + depth).min(host.max.x), y0 + width),
			}
		}
		1 => {
			let y0 = (dc.y - half_w).clamp(host.min.y, host.max.y - width);
			Aabb2d {
				min: Vec2::new((host.max.x - depth).max(host.min.x), y0),
				max: Vec2::new(host.max.x, y0 + width),
			}
		}
		2 => {
			let x0 = (dc.x - half_w).clamp(host.min.x, host.max.x - width);
			Aabb2d {
				min: Vec2::new(x0, host.min.y),
				max: Vec2::new(x0 + width, (host.min.y + depth).min(host.max.y)),
			}
		}
		_ => {
			let x0 = (dc.x - half_w).clamp(host.min.x, host.max.x - width);
			Aabb2d {
				min: Vec2::new(x0, (host.max.y - depth).max(host.min.y)),
				max: Vec2::new(x0 + width, host.max.y),
			}
		}
	};
	let rem = subtract_aabb2(host, &[entry]);
	Some((entry, rem))
}
