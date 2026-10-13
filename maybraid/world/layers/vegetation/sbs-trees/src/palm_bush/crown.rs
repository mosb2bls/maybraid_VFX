//! Stacked [`FrondCrownShape`] rings from a ground anchor (RFC §3.1.7.10).

use sbs_geometry::anchors::palm_bush::DEFAULT_HEIGHT;
use sbs_geometry::{FrondCrownShape, PalmBushSbs};

/// Lower-ring frond length as a fraction of `H` (RFC `0.25`).
const FROND_LENGTH_FRACTION_LO: f32 = 0.25;
/// Upper-ring frond length as a fraction of `H` (RFC `0.40`).
const FROND_LENGTH_FRACTION_HI: f32 = 0.40;
/// Rachis width as a fraction of `H` (RFC `0.05`).
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.05;

/// RFC-aligned frond crown defaults scaled to tree height `H`.
pub fn frond_shape_for_ring(
	geometry: &PalmBushSbs,
	ring: u32,
	foliage_seed: i32,
) -> FrondCrownShape {
	let h = geometry.height();
	let scale = geometry.frond_world_scale.max(1e-8);
	let proto = geometry.to_proto();
	let u = proto.ring_vertical_bias(ring);
	let length_fraction =
		FROND_LENGTH_FRACTION_LO + (FROND_LENGTH_FRACTION_HI - FROND_LENGTH_FRACTION_LO) * u;
	let downward_tilt = 0.38 + (1.0 - u) * 0.18;
	let emission_lift = 0.18 + u * 0.22;
	let h_scale = h / DEFAULT_HEIGHT.max(1e-6);
	let droop = (0.52 + (1.0 - u) * 0.16) * h_scale;
	let arch_lift = (0.22 + u * 0.12) * h_scale;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (length_fraction * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop,
		arch_lift,
		twist: 0.16,
		leaflet_count: 14,
		spine_segments: 10,
		shoot_half_radius: 0.014,
		rachis_half_thickness: 0.005,
		leaflet_length_scale: 3.4,
		downward_tilt_radians: downward_tilt,
		outward_spread_radians: 0.95,
		emission_lift_radians: emission_lift,
		seed: foliage_seed.wrapping_add(ring as i32),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lower_rings_droop_more_than_upper() {
		let geometry = PalmBushSbs::default();
		let low = frond_shape_for_ring(&geometry, 0, 0);
		let high = frond_shape_for_ring(&geometry, geometry.crown.ring_count - 1, 0);
		assert!(low.droop > high.droop);
		assert!(low.downward_tilt_radians > high.downward_tilt_radians);
	}

	#[test]
	fn droop_and_arch_scale_with_height() {
		let tall = PalmBushSbs::default();
		let mut unit = tall.clone();
		unit.scale.height = 1.0;
		let t = frond_shape_for_ring(&tall, 0, 0);
		let u = frond_shape_for_ring(&unit, 0, 0);
		assert!((t.droop / t.length - u.droop / u.length).abs() < 1e-4);
		assert!((t.arch_lift / t.length - u.arch_lift / u.length).abs() < 1e-4);
	}
}
