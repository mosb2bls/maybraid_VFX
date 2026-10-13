//! Stacked [`FrondCrownShape`] rings at the trunk tip (RFC palm crown).

use sbs_geometry::anchors::date_palm::DEFAULT_STALK_HEIGHT;
use sbs_geometry::{DatePalmSbs, FrondCrownShape};

/// Rachis width in world units before crown uniform scale.
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.070;
/// Lower-ring frond length as a fraction of `H` (RFC `0.25`).
const FROND_LENGTH_FRACTION_LO: f32 = 0.6;
/// Upper-ring frond length as a fraction of `H` (RFC `0.40`).
const FROND_LENGTH_FRACTION_HI: f32 = 0.8;

/// RFC-aligned frond crown defaults scaled to tree height `H`.
pub fn frond_shape_for_ring(
	geometry: &DatePalmSbs,
	ring: u32,
	foliage_seed: i32,
) -> FrondCrownShape {
	let h = geometry.height();
	let scale = geometry.frond_world_scale.max(1e-8);
	let proto = geometry.to_proto();
	let u = proto.ring_vertical_bias(ring);
	let downward_tilt = 0.44 + (1.0 - u) * 0.20;
	let emission_lift = 0.28 + (1.0 - u) * 0.10;
	// Leftover meters authored for [`DEFAULT_STALK_HEIGHT`]. Divide by
	// `frond_world_scale` like length — grove variants (Palm Shade ~0.47) must
	// keep the same droop/length as `/show` after `world_space_frond_shape`.
	// Lower rings weep; upper stay milder so they can still rise.
	let h_scale = h / DEFAULT_STALK_HEIGHT.max(1e-6);
	let droop = (1.7 + (1.0 - u) * 0.85) * h_scale / scale;
	let arch_lift = 0.30 * h_scale / scale;
	// length shortens with ring index
	let length_fraction =
		FROND_LENGTH_FRACTION_LO + (FROND_LENGTH_FRACTION_HI - FROND_LENGTH_FRACTION_LO) * u;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (length_fraction * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop,
		arch_lift,
		twist: 0.66,
		leaflet_count: 16,
		spine_segments: 11,
		shoot_half_radius: 0.020,
		rachis_half_thickness: 0.007,
		leaflet_length_scale: 4.0,
		downward_tilt_radians: downward_tilt,
		outward_spread_radians: 2.0,
		emission_lift_radians: emission_lift,
		seed: foliage_seed.wrapping_add(ring as i32),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn droop_and_arch_scale_with_height() {
		let tall = DatePalmSbs::default();
		let mut unit = tall.clone();
		unit.scale.stalk_height = 1.0;
		let t = frond_shape_for_ring(&tall, 0, 0);
		let u = frond_shape_for_ring(&unit, 0, 0);
		assert!((t.droop / t.length - u.droop / u.length).abs() < 1e-4);
		assert!((t.arch_lift / t.length - u.arch_lift / u.length).abs() < 1e-4);
	}

	#[test]
	fn lower_rings_droop_more_than_upper() {
		let geometry = DatePalmSbs::default();
		let low = frond_shape_for_ring(&geometry, 0, 0);
		let high = frond_shape_for_ring(&geometry, geometry.crown.ring_count - 1, 0);
		assert!(low.droop > high.droop);
		assert!(
			low.droop / low.length > 0.24,
			"lower date fronds should weep, got {}",
			low.droop / low.length
		);
	}

	#[test]
	fn world_space_droop_ratio_ignores_frond_world_scale() {
		use crate::palm_tree::world_space_frond_shape;

		let show = DatePalmSbs::default();
		let mut grove = DatePalmSbs::default();
		grove.scale.stalk_height = 1.0;
		grove.frond_world_scale = 0.35 + 0.35 * 0.5;
		let s = world_space_frond_shape(frond_shape_for_ring(&show, 0, 0), show.frond_world_scale);
		let g =
			world_space_frond_shape(frond_shape_for_ring(&grove, 0, 0), grove.frond_world_scale);
		assert!((s.droop / s.length - g.droop / g.length).abs() < 1e-4);
		assert!(
			g.droop / g.length > 0.35,
			"unit grove mesh should weep, got {}",
			g.droop / g.length
		);
	}
}
