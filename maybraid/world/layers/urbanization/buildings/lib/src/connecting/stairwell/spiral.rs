//! Circular flight inside an exclusive [`WellAabb`].
//!
//! Inscribe a circle so the outer rail stays in the box. First tread at the
//! walk-on azimuth, last at the walk-off. The walk-off landing is a door strip
//! authored first; the last leading arrives on that strip. One circuit from
//! walk-on to walk-off (wrapping coefficient at most one).

use std::f32::consts::TAU;

use bevy_math::Vec2;
use building_components::panels::PanelStyle;
use building_components::placed::Placement;
use building_components::stairs::{Stair, StairNode, StraightStair};

use super::laws::{resolved_rise, tread_count, MIN_LANDING};
use super::well::{yaw_xz, WellAabb};
use super::Fit;

const MIN_RADIUS: f32 = 0.2;

/// Circular nodes + the walk-off landing (door strip, not a sheared pad).
pub(crate) fn fit(well: &WellAabb, style: PanelStyle, thickness: f32) -> Fit {
	let rise = resolved_rise(well.rise());
	let width = well.tread_width();
	let radius = (well.half_min() - MIN_LANDING - 0.5 * width).max(MIN_RADIUS);
	let n = tread_count(rise);
	let turns = spiral_turns(well);
	let intervals = n.saturating_sub(1).max(1);
	let going = (turns * TAU * radius) / intervals as f32;
	let center = well.center_xz();
	let start_yaw = yaw_xz(well.walk_on.into_xz());
	let stairs =
		circular_nodes(center, well.bottom_y(), start_yaw, radius, width, going, rise, n, turns);
	let depth = width + MIN_LANDING;
	Fit {
		stairs,
		door: Some(well.walk_off_landing_strip(style, thickness, depth)),
		mids: Vec::new(),
	}
}

fn spiral_turns(well: &WellAabb) -> f32 {
	let start = yaw_xz(well.walk_on.into_xz());
	let end = yaw_xz(well.walk_off.into_xz());
	let mut sweep = wrap_ccw(end - start);
	if sweep < 0.2 * TAU {
		sweep += TAU;
	}
	(sweep / TAU).min(1.0)
}

fn circular_nodes(
	center: Vec2,
	base_y: f32,
	start_yaw: f32,
	radius: f32,
	width: f32,
	going: f32,
	rise: f32,
	n: u32,
	turns: f32,
) -> Vec<StairNode> {
	if n == 0 {
		return Vec::new();
	}
	let sweep = turns.max(1e-4) * TAU;
	let rise_step = rise / n as f32;
	let (ys, yc) = start_yaw.sin_cos();
	let rotate = |lx: f32, lz: f32| Vec2::new(yc * lx + ys * lz, -ys * lx + yc * lz);

	(0..n)
		.map(|i| {
			let t = if n == 1 { 1.0 } else { i as f32 / (n - 1) as f32 };
			let theta = t * sweep;
			let (s, c) = theta.sin_cos();
			let p = center + rotate(c * radius, -s * radius);
			let travel_yaw = start_yaw + theta + std::f32::consts::FRAC_PI_2;
			let y = base_y + i as f32 * rise_step;
			StairNode::rough_stone(
				Stair::Straight(
					StraightStair::single(width, going.max(1e-4), rise_step)
						.with_flush_start(false),
				),
				Placement::new(bevy_math::Vec3::new(p.x, y, p.y), travel_yaw),
			)
		})
		.collect()
}

fn wrap_ccw(delta: f32) -> f32 {
	let mut d = delta;
	while d <= 0.0 {
		d += TAU;
	}
	d
}
