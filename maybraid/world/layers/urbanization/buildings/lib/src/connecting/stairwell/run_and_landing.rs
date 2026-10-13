//! Run-and-landing flight inside an exclusive [`WellAabb`].
//!
//! Every flight is an **I**: hug the wall, fill one half so the other half can
//! tile. Extra laps for going add an interior landing and run back the other
//! half when [`super::laws::MIN_GOING`] and [`super::laws::MIN_HEADROOM`]
//! allow. If the last I would still need a U to the walk-off, try **one**
//! routing switchback first. Interior and L/U pads share a walkable minimum
//! ([`MIN_WALK_LANDING`]), capped so two pads still leave a real I. A door
//! strip stays thin only when the last I already ends on the walk-off.

use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelStyle;
use building_components::placed::Placement;
use building_components::stairs::{Stair, StairNode, StraightStair};

use crate::paneling::quad_panel::QuadPanel;

use super::laws::{
	add_laps_for_going, resolved_rise, tread_count, MIN_GOING, MIN_LANDING, MIN_RUN,
};
use super::well::{yaw_xz, WellAabb};
use super::{Fit, WellSide};

/// Smallest walkable interior / L / U landing (meters). Not “as deep as wide.”
const MIN_WALK_LANDING: f32 = 0.9;

/// Which half of the walk-on split the I occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Half {
	/// Toward [`WellSide::ccw_next`] of the walk-on (first I).
	Ccw,
	Cw,
}

impl Half {
	fn hug(self, walk_on: WellSide) -> WellSide {
		match self {
			Self::Ccw => walk_on.ccw_next(),
			Self::Cw => walk_on.cw_next(),
		}
	}
}

struct Flight {
	half: Half,
	travel: Vec2,
	start_side: WellSide,
	end_side: WellSide,
	k: u32,
	start_pad: f32,
	end_pad: f32,
}

/// Half-well I flights + interior turnarounds + walk-off strip path.
pub(crate) fn fit(well: &WellAabb, style: PanelStyle, thickness: f32, want_landing: bool) -> Fit {
	let rise = resolved_rise(well.rise());
	let width = i_width(well);
	let n = tread_count(rise);
	let door = door_pad_m(well);
	let turn = turn_pad_m(well);
	let along = i_along(well);
	let run = (along - door - turn).max(MIN_RUN);
	let laps = route_lap(lap_count(n, run, rise), n, well.walk_on, well.walk_off);
	let counts = split_treads(n, laps);
	let flights = flights(well, &counts, door, turn);
	let rise_step = rise / n as f32;
	let mut y = well.bottom_y();
	let mut stairs = Vec::new();
	let mut mids = Vec::new();
	for (i, flight) in flights.iter().enumerate() {
		if flight.k == 0 {
			continue;
		}
		let run = (along - flight.start_pad - flight.end_pad).max(MIN_RUN);
		let going = (run / flight.k as f32).max(1e-4);
		stairs.push(flight_node(well, flight, width, going, rise_step, y));
		y += flight.k as f32 * rise_step;
		let last = i + 1 == flights.len();
		if !last {
			mids.push(well.face_strip(flight.end_side, y, flight.end_pad, style, thickness));
		}
	}
	let landing = if want_landing {
		if let Some(last) = flights.iter().rev().find(|f| f.k > 0) {
			let (door, extras) = walk_off_pads(well, last, door, turn, style, thickness);
			mids.extend(extras);
			Some(door)
		} else {
			Some(well.walk_off_landing_strip(style, thickness, door))
		}
	} else {
		None
	};
	Fit { stairs, door: landing, mids }
}

fn i_width(well: &WellAabb) -> f32 {
	well.face_half(well.walk_on).max(1e-4)
}

fn i_along(well: &WellAabb) -> f32 {
	2.0 * well.face_half(well.walk_on.ccw_next())
}

fn door_pad_m(well: &WellAabb) -> f32 {
	MIN_GOING.min(i_along(well) * 0.2).max(MIN_LANDING)
}

/// Walkable turn / L / U depth. Shrinks when two pads would eat the I.
fn turn_pad_m(well: &WellAabb) -> f32 {
	let along = i_along(well);
	let door = door_pad_m(well);
	let two_turn = ((along - MIN_RUN) * 0.5).max(door);
	let door_turn = (along - door - MIN_RUN).max(door);
	let cap = two_turn.min(door_turn);
	MIN_WALK_LANDING.max(door).min(cap)
}

fn lap_count(n: u32, run: f32, rise: f32) -> u32 {
	add_laps_for_going(1, rise, |laps| (laps as f32 * run) / n.max(1) as f32)
}

/// One extra I when the last flight would finish opposite the walk-off (a U).
fn route_lap(laps: u32, n: u32, walk_on: WellSide, walk_off: WellSide) -> u32 {
	let end = if laps % 2 == 1 { walk_on.opposite() } else { walk_on };
	if end.opposite() == walk_off && n > laps {
		laps + 1
	} else {
		laps
	}
}

fn split_treads(n: u32, laps: u32) -> Vec<u32> {
	let laps = laps.max(1);
	let base = n / laps;
	let rem = n % laps;
	(0..laps).map(|i| base + u32::from(i < rem)).collect()
}

fn flights(well: &WellAabb, counts: &[u32], door: f32, turn: f32) -> Vec<Flight> {
	let inward = -well.walk_on.into_xz();
	let far = well.walk_on.opposite();
	let n = counts.len();
	let last_end = if n % 2 == 1 { far } else { well.walk_on };
	let last_lu = last_end != well.walk_off;
	counts
		.iter()
		.enumerate()
		.map(|(i, &k)| {
			let outbound = i % 2 == 0;
			Flight {
				half: if outbound { Half::Ccw } else { Half::Cw },
				travel: if outbound { inward } else { -inward },
				start_side: if outbound { well.walk_on } else { far },
				end_side: if outbound { far } else { well.walk_on },
				k,
				start_pad: if i == 0 { door } else { turn },
				end_pad: if i + 1 == n {
					if last_lu {
						turn
					} else {
						door
					}
				} else {
					turn
				},
			}
		})
		.collect()
}

/// Strip path from the last I's end wall to walk-off, avoiding a lid on the I.
fn connect_sides(end: WellSide, off: WellSide, avoid_hug: WellSide) -> Vec<WellSide> {
	if end == off {
		return vec![end];
	}
	let ccw = walk(end, off, WellSide::ccw_next);
	let cw = walk(end, off, WellSide::cw_next);
	let hits = |path: &[WellSide]| path.iter().skip(1).any(|&s| s == avoid_hug && s != off);
	match (hits(&ccw), hits(&cw)) {
		(true, false) => cw,
		(false, true) => ccw,
		_ if ccw.len() <= cw.len() => ccw,
		_ => cw,
	}
}

fn walk(start: WellSide, end: WellSide, step: fn(WellSide) -> WellSide) -> Vec<WellSide> {
	let mut path = vec![start];
	let mut s = start;
	while s != end {
		s = step(s);
		path.push(s);
		if path.len() > 4 {
			break;
		}
	}
	path
}

fn flight_node(
	well: &WellAabb,
	flight: &Flight,
	width: f32,
	going: f32,
	rise_step: f32,
	y: f32,
) -> StairNode {
	let start = first_tread_xz(well, flight, going);
	let length = flight.k as f32 * going;
	let height = flight.k as f32 * rise_step;
	let mut geom = StraightStair::run(height, length, width, going).with_flush_start(true);
	geom.tread_height = rise_step.max(1e-4);
	StairNode::rough_stone(
		Stair::Straight(geom),
		Placement::new(Vec3::new(start.x, y, start.y), yaw_xz(flight.travel)),
	)
}

fn first_tread_xz(well: &WellAabb, flight: &Flight, going: f32) -> Vec2 {
	let lateral = half_center(well, flight.half);
	let start_mid = well.side_mid(flight.start_side, 0.0);
	let start_edge =
		Vec2::new(start_mid.x, start_mid.z) - flight.start_side.into_xz() * flight.start_pad;
	let t = unit(flight.travel);
	let n = Vec2::new(-t.y, t.x);
	let p = start_edge + t * (0.5 * going);
	t * p.dot(t) + n * lateral.dot(n)
}

fn half_center(well: &WellAabb, half: Half) -> Vec2 {
	let c = well.center_xz();
	let hug = half.hug(well.walk_on).into_xz();
	let ext = if hug.x.abs() > hug.y.abs() { well.half_x() } else { well.half_z() };
	c + hug * (0.5 * ext)
}

fn unit(v: Vec2) -> Vec2 {
	if v.length_squared() < 1e-8 {
		Vec2::Y
	} else {
		v.normalize()
	}
}

fn walk_off_pads(
	well: &WellAabb,
	last: &Flight,
	door: f32,
	turn: f32,
	style: PanelStyle,
	thickness: f32,
) -> (QuadPanel, Vec<QuadPanel>) {
	if last.end_side == well.walk_off {
		return (door_slab(well, last, door, style, thickness), Vec::new());
	}
	let hug = last.half.hug(well.walk_on);
	let path = connect_sides(last.end_side, well.walk_off, hug);
	let extras = path
		.iter()
		.copied()
		.filter(|s| *s != well.walk_off)
		.map(|side| well.face_strip(side, well.top_y(), turn, style, thickness))
		.collect();
	if well.walk_off == hug {
		(
			half_face_strip(
				well,
				well.walk_off,
				last.end_side,
				well.top_y(),
				turn,
				style,
				thickness,
			),
			extras,
		)
	} else {
		(well.face_strip(well.walk_off, well.top_y(), turn, style, thickness), extras)
	}
}

fn door_slab(
	well: &WellAabb,
	last: &Flight,
	pad: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let hug = last.half.hug(well.walk_on);
	if well.walk_off == hug {
		half_face_strip(well, well.walk_off, last.end_side, well.top_y(), pad, style, thickness)
	} else {
		well.walk_off_landing_strip(style, thickness, pad)
	}
}

fn half_face_strip(
	well: &WellAabb,
	side: WellSide,
	toward: WellSide,
	y: f32,
	depth: f32,
	style: PanelStyle,
	thickness: f32,
) -> QuadPanel {
	let (lo, hi) = half_along(well, side, toward);
	well.strip_slab(side, y, depth, lo, hi, style, thickness)
}

fn half_along(well: &WellAabb, side: WellSide, toward: WellSide) -> (f32, f32) {
	let (lo, hi) = well.face_along(side);
	let mid = 0.5 * (lo + hi);
	let toward_xz = toward.into_xz();
	let sign = match side {
		WellSide::NegZ | WellSide::PosZ => toward_xz.x,
		WellSide::NegX | WellSide::PosX => toward_xz.y,
	};
	if sign >= 0.0 {
		(mid, hi)
	} else {
		(lo, mid)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn same_side_is_a_u_around_the_unused_half() {
		let path = connect_sides(WellSide::PosZ, WellSide::NegZ, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::NegX, WellSide::NegZ]);
	}

	#[test]
	fn unused_adjacent_is_an_l() {
		let path = connect_sides(WellSide::PosZ, WellSide::NegX, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::NegX]);
	}

	#[test]
	fn hug_adjacent_is_an_l_on_the_end_corner() {
		let path = connect_sides(WellSide::PosZ, WellSide::PosX, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ, WellSide::PosX]);
	}

	#[test]
	fn already_on_the_walk_off_wall_is_just_the_door() {
		let path = connect_sides(WellSide::PosZ, WellSide::PosZ, WellSide::PosX);
		assert_eq!(path, vec![WellSide::PosZ]);
	}

	#[test]
	fn return_i_same_side_is_just_the_door() {
		let path = connect_sides(WellSide::NegZ, WellSide::NegZ, WellSide::NegX);
		assert_eq!(path, vec![WellSide::NegZ]);
	}

	#[test]
	fn u_situation_adds_one_routing_lap() {
		assert_eq!(route_lap(1, 17, WellSide::NegZ, WellSide::NegZ), 2);
		assert_eq!(route_lap(1, 17, WellSide::NegZ, WellSide::PosZ), 1);
		assert_eq!(route_lap(1, 1, WellSide::NegZ, WellSide::NegZ), 1);
	}

	#[test]
	fn turn_pad_is_walkable_min_not_i_width() {
		let wide = WellAabb::from_plan(
			Vec3::new(-1.2, 0.0, -1.2),
			Vec3::new(1.2, 3.0, 1.2),
			WellSide::NegZ,
			WellSide::NegZ,
			0.4,
		);
		assert!((turn_pad_m(&wide) - MIN_WALK_LANDING).abs() < 0.02);
		let two = 2.0 * turn_pad_m(&wide);
		assert!(i_along(&wide) - two + 1e-3 >= MIN_RUN);

		let tiny = WellAabb::from_plan(
			Vec3::new(-0.6, 0.0, -0.6),
			Vec3::new(0.6, 1.5, 0.6),
			WellSide::NegZ,
			WellSide::NegZ,
			0.4,
		);
		assert!(turn_pad_m(&tiny) < MIN_WALK_LANDING);
		assert!(i_along(&tiny) - 2.0 * turn_pad_m(&tiny) + 1e-3 >= MIN_RUN);
	}
}
