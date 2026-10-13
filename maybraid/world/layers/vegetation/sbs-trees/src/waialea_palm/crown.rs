//! Stacked [`FrondCrownShape`] rings above the trunk tip (RFC Waialea crown).

use sbs_geometry::anchors::waialea_palm::DEFAULT_STALK_HEIGHT;
use sbs_geometry::{FrondCrownShape, WaialeaPalmSbs};

/// Lower-ring frond length as a fraction of `H` (RFC `0.25`).
const FROND_LENGTH_FRACTION_LO: f32 = 0.25;
/// Upper-ring frond length as a fraction of `H` (RFC `0.40`).
const FROND_LENGTH_FRACTION_HI: f32 = 0.40;
/// Rachis width as a fraction of `H` (RFC `0.05`).
const FROND_WIDTH_FRACTION_OF_HEIGHT: f32 = 0.05;

pub fn frond_shape_for_ring(
	geometry: &WaialeaPalmSbs,
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
	// Leftover meters authored for [`DEFAULT_STALK_HEIGHT`]; scale with `H` so unitize
	// keeps `/show` droop/length (placement scale restores world size).
	let h_scale = h / DEFAULT_STALK_HEIGHT.max(1e-6);
	let droop = (0.72 + (1.0 - u) * 0.16) * h_scale;
	let arch_lift = (0.22 + u * 0.12) * h_scale;

	FrondCrownShape {
		frond_count: proto.fronds_per_ring,
		length: (length_fraction * h) / scale,
		width: (FROND_WIDTH_FRACTION_OF_HEIGHT * h) / scale,
		droop,
		arch_lift,
		twist: 0.66,
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
	fn droop_and_arch_scale_with_height() {
		let tall = WaialeaPalmSbs::default();
		let mut unit = tall.clone();
		unit.scale.stalk_height = 1.0;
		let t = frond_shape_for_ring(&tall, 0, 0);
		let u = frond_shape_for_ring(&unit, 0, 0);
		assert!((t.droop / t.length - u.droop / u.length).abs() < 1e-4);
		assert!((t.arch_lift / t.length - u.arch_lift / u.length).abs() < 1e-4);
		assert!((u.droop - t.droop / DEFAULT_STALK_HEIGHT).abs() < 1e-4);
	}
}
