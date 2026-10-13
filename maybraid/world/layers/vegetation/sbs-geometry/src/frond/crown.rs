//! Crown-level frond direction caps (palm / fern clusters).

use bevy_math::{Mat3, Quat, Vec3};

use crate::jitter::{signed_jitter, unit_jitter};

const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

/// Lane block reserved for [`length_scale`] so it never reuses a direction lane.
const LENGTH_LANE_BASE: u32 = 0x8000_0000;

/// Outward frond headings: horizontal fan with separate pitch (lift / droop) and azimuth spread.
///
/// `outward_spread_radians` wobbles azimuth only; it is not added into the downward pitch (avoids
/// the “flanged” look where high spread reads as near-horizontal emission plus steep local droop).
pub fn crown_directions(
	count: u32,
	seed: i32,
	downward_tilt_radians: f32,
	outward_spread_radians: f32,
	emission_lift_radians: f32,
) -> Vec<Vec3> {
	let n = count.max(1);
	let phase = unit_jitter(seed, 0) * std::f32::consts::TAU;
	(0..n)
		.map(|i| {
			let fi = i as f32;
			let down = downward_tilt_radians * (0.65 + 0.35 * unit_jitter(seed, 4 * i + 1));
			let spread = outward_spread_radians * (0.4 + 0.6 * unit_jitter(seed, 4 * i + 2));
			let lift = emission_lift_radians * (0.7 + 0.3 * unit_jitter(seed, 4 * i + 3));
			let pitch = lift - down;
			let azimuth = GOLDEN_ANGLE.mul_add(fi, phase) + spread * signed_jitter(seed, 4 * i + 4);
			Vec3::new(pitch.cos() * azimuth.cos(), pitch.sin(), pitch.cos() * azimuth.sin())
				.normalize_or_zero()
		})
		.collect()
}

/// Per-frond length multiplier in `[min, max]` (deterministic from seed).
pub fn length_scale(index: u32, seed: i32, min: f32, max: f32) -> f32 {
	min + (max - min) * unit_jitter(seed, LENGTH_LANE_BASE | index)
}

/// Align frond-local +X to the crown emission direction.
///
/// [`Quat::from_rotation_arc`] leaves twist free: for some azimuths, local −Y (spine
/// droop) rolls into the XZ plane and the blade reads as a straight stick. Build a
/// world-up frame so droop stays in the vertical plane of the emission heading.
/// Parallel-to-Y emission (no unique vertical plane) falls back to the arc.
pub fn align_frond_direction(direction: Vec3) -> Quat {
	let forward = direction.normalize_or_zero();
	if forward.length_squared() < 1e-12 {
		return Quat::IDENTITY;
	}
	let right = forward.cross(Vec3::Y);
	if right.length_squared() < 1e-8 {
		return Quat::from_rotation_arc(Vec3::X, forward);
	}
	let right = right.normalize();
	let up = right.cross(forward).normalize_or_zero();
	Quat::from_mat3(&Mat3::from_cols(forward, up, right))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spread_does_not_flatten_emission_to_pure_horizontal() {
		let steep = crown_directions(1, 0, 0.8, 0.0, 0.0)[0];
		let with_spread = crown_directions(1, 0, 0.8, 1.2, 0.0)[0];
		// Jitter band floor is sin(0.65 * 0.8) ≈ 0.497.
		assert!(steep.y < -0.45, "expected downward pitch: {steep:?}");
		assert!(with_spread.y < -0.4, "spread should not cancel downward pitch: {with_spread:?}");
	}

	#[test]
	fn emission_lift_can_point_outward_and_up() {
		let d = crown_directions(1, 0, 0.15, 0.5, 0.35)[0];
		assert!(d.y > 0.09, "expected lifted emission: {d:?}");
	}

	/// Grove placement seeds land near `2^30`, where the old `seed + i as f32` jitter
	/// collapsed every frond (and every ring, salted 18 apart) onto identical values.
	#[test]
	fn per_frond_variation_survives_large_seeds() {
		let seed = 1_073_127_521;
		let dirs = crown_directions(12, seed, 0.6, 0.95, 0.2);
		let min_y = dirs.iter().map(|d| d.y).fold(f32::INFINITY, f32::min);
		let max_y = dirs.iter().map(|d| d.y).fold(f32::NEG_INFINITY, f32::max);
		assert!(max_y - min_y > 1e-3, "fronds collapsed to one pitch: {dirs:?}");

		let lengths: Vec<f32> = (0..12).map(|i| length_scale(i, seed, 0.82, 1.08)).collect();
		let min_l = lengths.iter().copied().fold(f32::INFINITY, f32::min);
		let max_l = lengths.iter().copied().fold(f32::NEG_INFINITY, f32::max);
		assert!(max_l - min_l > 1e-3, "lengths collapsed: {lengths:?}");
	}

	#[test]
	fn droop_axis_stays_in_the_vertical_plane_for_every_azimuth() {
		let pitch: f32 = 0.12;
		for i in 0..16 {
			let az = i as f32 * std::f32::consts::TAU / 16.0;
			let dir = Vec3::new(pitch.cos() * az.cos(), pitch.sin(), pitch.cos() * az.sin())
				.normalize_or_zero();
			let q = align_frond_direction(dir);
			assert!((q * Vec3::X - dir).length() < 1e-4, "az {i}: +X missed emission");
			let droop = q * Vec3::NEG_Y;
			assert!(droop.y < -0.7, "az {i}: droop rolled sideways/out: {droop:?}");
		}
	}

	/// Stacked palm rings salt seeds 18 apart; adjacent rings must not render identically.
	#[test]
	fn adjacent_ring_seeds_decorrelate_at_large_magnitude() {
		let seed = 1_073_127_521;
		let a = crown_directions(12, seed, 0.6, 0.95, 0.2);
		let b = crown_directions(12, seed + 18, 0.6, 0.95, 0.2);
		assert!(
			a.iter().zip(&b).any(|(p, q)| (*p - *q).length() > 1e-3),
			"rings rendered identically"
		);
	}
}
