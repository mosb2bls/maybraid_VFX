//! Shared exclusive-well stair laws.

use building_components::stairs::StraightStair;

/// Smallest walkable going (meters). A tight going is accepted rather than a
/// second circuit — wrapping stays at one lap.
pub(crate) const MIN_GOING: f32 = 0.25;
/// Smallest rise per lap (meters). Used when a well is allowed more than one
/// lap; circuit wrapping itself is capped at one.
pub(crate) const MIN_HEADROOM: f32 = 2.0;
/// Smallest door / rim strip (meters).
pub(crate) const MIN_LANDING: f32 = 0.12;
/// Smallest leftover I run after pads (meters).
pub(crate) const MIN_RUN: f32 = 0.08;

pub(crate) fn resolved_rise(rise: f32) -> f32 {
	rise.max(StraightStair::DEFAULT_TREAD_HEIGHT)
}

pub(crate) fn tread_count(rise: f32) -> u32 {
	(resolved_rise(rise) / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32
}

/// Keep the authored circuit. Do not pre-determine a wrapping coefficient
/// above one — a storey taller than `2 · MIN_HEADROOM` used to unlock a second
/// lap just to protect [`MIN_GOING`], which stacked the same walls twice.
pub(crate) fn add_laps_for_going(laps: u32, _rise: f32, _going_of: impl Fn(u32) -> f32) -> u32 {
	laps.max(1).min(1)
}

#[allow(dead_code)]
pub(crate) fn headroom_allows(rise: f32, laps: f32) -> bool {
	rise / laps.max(1e-4) + 1e-4 >= MIN_HEADROOM
}
