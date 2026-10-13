//! Rectangular flight inside an exclusive [`WellAabb`].
//!
//! Hug each wall CCW. Corner pads are axis-aligned and at least as wide as the
//! tread so the last leading meets the next flight. A circuit wall with no
//! treads still gets a full-face strip so two runs are not stranded across a
//! gap. The walk-off landing is a door strip. One circuit from walk-on to
//! walk-off. A skinny well may collapse the hole.

use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelStyle;
use building_components::placed::Placement;
use building_components::stairs::{Stair, StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;

use super::laws::{add_laps_for_going, resolved_rise, tread_count, MIN_LANDING, MIN_RUN};
use super::well::WellAabb;
use super::{Fit, WellSide};

/// Wall-hugging flights + corner pads + walk-off strip.
pub(crate) fn fit(well: &WellAabb, style: PanelStyle, thickness: f32) -> Fit {
	let rise = resolved_rise(well.rise());
	let width = well.tread_width();
	let n = tread_count(rise);
	let pad = corner_pad_m(well, width);
	let sides = circuit_sides(well, n, rise, pad);
	let runs: Vec<f32> = sides.iter().map(|s| wall_run(well, *s, pad)).collect();
	let counts = split_treads_by_run(n, &runs);
	let rise_step = rise / n as f32;
	let mut y = well.bottom_y();
	let mut stairs = Vec::new();
	let mut corners = Vec::new();
	for (i, side) in sides.iter().copied().enumerate() {
		let k = counts[i];
		if k > 0 {
			// Last leading sits on the end pad: going fills this wall, not a global share.
			let going = (runs[i] / k as f32).max(1e-4);
			stairs.push(flight_node(well, side, pad, width, going, rise_step, k, y));
			y += k as f32 * rise_step;
		}
		let last = i + 1 == sides.len();
		if !last {
			if k == 0 {
				// No treads on this wall: a full-face strip so the next flight is
				// not stranded across a gap.
				corners.push(well.face_strip(side, y, pad, style, thickness));
			} else {
				corners.push(corner_slab(well, side, y, pad, style, thickness));
			}
		}
	}
	let depth = width + MIN_LANDING;
	Fit { stairs, door: Some(well.walk_off_landing_strip(style, thickness, depth)), mids: corners }
}

#[cfg(test)]
pub(super) fn flight_end_leading(well: &WellAabb, side: WellSide) -> Vec2 {
	let pad = corner_pad_m(well, well.tread_width());
	let min = well.min();
	let max = well.max();
	let half_w = 0.5 * well.tread_width();
	match side {
		WellSide::NegZ => Vec2::new(max.x - pad, min.z + half_w),
		WellSide::PosX => Vec2::new(max.x - half_w, max.z - pad),
		WellSide::PosZ => Vec2::new(min.x + pad, max.z - half_w),
		WellSide::NegX => Vec2::new(min.x + half_w, min.z + pad),
	}
}

fn corner_pad_m(well: &WellAabb, width: f32) -> f32 {
	let shortest = (2.0 * well.half_x()).min(2.0 * well.half_z());
	// Match the tread width so the turn pad actually meets both flights. Cap so
	// each wall still keeps a leftover I ([`MIN_RUN`]).
	let max_pad = ((shortest - MIN_RUN) * 0.5).max(MIN_LANDING);
	width.min(max_pad).max(MIN_LANDING)
}

fn wall_run(well: &WellAabb, side: WellSide, pad: f32) -> f32 {
	(2.0 * well.face_half(side) - 2.0 * pad).max(MIN_RUN)
}

fn path_sides(on: WellSide, off: WellSide) -> Vec<WellSide> {
	if on == off {
		return vec![on, on.ccw_next(), on.opposite(), on.cw_next()];
	}
	let mut sides = vec![on];
	let mut s = on;
	while s.ccw_next() != off {
		s = s.ccw_next();
		sides.push(s);
		if sides.len() >= 4 {
			break;
		}
	}
	sides
}

fn circuit_sides(well: &WellAabb, n: u32, rise: f32, pad: f32) -> Vec<WellSide> {
	let path = path_sides(well.walk_on, well.walk_off);
	let path_run: f32 = path.iter().map(|s| wall_run(well, *s, pad)).sum();
	let laps = add_laps_for_going(1, rise, |laps| laps as f32 * path_run / n.max(1) as f32);
	(0..laps).flat_map(|_| path.iter().copied()).collect()
}

/// Treads in proportion to wall run so no flight overshoots its pads.
fn split_treads_by_run(n: u32, runs: &[f32]) -> Vec<u32> {
	if runs.is_empty() {
		return Vec::new();
	}
	let total = runs.iter().sum::<f32>().max(1e-4);
	let mut counts: Vec<u32> =
		runs.iter().map(|r| ((n as f32) * (*r / total)).floor() as u32).collect();
	let used: u32 = counts.iter().sum();
	let mut rem = n.saturating_sub(used);
	let mut order: Vec<usize> = (0..runs.len()).collect();
	order.sort_by(|&a, &b| {
		let fa = n as f32 * runs[a] / total - counts[a] as f32;
		let fb = n as f32 * runs[b] / total - counts[b] as f32;
		fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
	});
	for i in order {
		if rem == 0 {
			break;
		}
		counts[i] += 1;
		rem -= 1;
	}
	counts
}

fn flight_node(
	well: &WellAabb,
	side: WellSide,
	pad: f32,
	width: f32,
	going: f32,
	rise_step: f32,
	k: u32,
	y: f32,
) -> StairNode {
	let start = first_tread_xz(well, side, pad, width, going);
	let length = k as f32 * going;
	let height = k as f32 * rise_step;
	let mut geom = StraightStair::run(height, length, width, going).with_flush_start(true);
	geom.tread_height = rise_step.max(1e-4);
	StairNode::rough_stone(
		Stair::Straight(geom),
		Placement::new(Vec3::new(start.x, y, start.y), side.travel_yaw()),
	)
}

fn first_tread_xz(well: &WellAabb, side: WellSide, pad: f32, width: f32, going: f32) -> Vec2 {
	let min = well.min();
	let max = well.max();
	let half_g = 0.5 * going;
	let half_w = 0.5 * width;
	match side {
		WellSide::NegZ => Vec2::new(min.x + pad + half_g, min.z + half_w),
		WellSide::PosX => Vec2::new(max.x - half_w, min.z + pad + half_g),
		WellSide::PosZ => Vec2::new(max.x - pad - half_g, max.z - half_w),
		WellSide::NegX => Vec2::new(min.x + half_w, max.z - pad - half_g),
	}
}

fn corner_slab(
	well: &WellAabb,
	from: WellSide,
	y: f32,
	pad: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let min = well.min();
	let max = well.max();
	let p = pad.max(MIN_LANDING);
	let (a0, a1, b0, b1) = match from {
		WellSide::NegZ => (
			Vec3::new(max.x - p, y, min.z),
			Vec3::new(max.x, y, min.z),
			Vec3::new(max.x - p, y, min.z + p),
			Vec3::new(max.x, y, min.z + p),
		),
		WellSide::PosX => (
			Vec3::new(max.x - p, y, max.z - p),
			Vec3::new(max.x, y, max.z - p),
			Vec3::new(max.x - p, y, max.z),
			Vec3::new(max.x, y, max.z),
		),
		WellSide::PosZ => (
			Vec3::new(min.x, y, max.z - p),
			Vec3::new(min.x + p, y, max.z - p),
			Vec3::new(min.x, y, max.z),
			Vec3::new(min.x + p, y, max.z),
		),
		WellSide::NegX => (
			Vec3::new(min.x, y, min.z),
			Vec3::new(min.x + p, y, min.z),
			Vec3::new(min.x, y, min.z + p),
			Vec3::new(min.x + p, y, min.z + p),
		),
	};
	QuadPanel::slab(style, a0, a1, b0, b1, thickness)
}
