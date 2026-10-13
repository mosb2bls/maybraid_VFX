//! **Frond crown** — arching leaflet chains for palms and ferns.

mod config;
mod crown;
mod spine;

use bevy_math::Vec3;

pub use config::FrondConfig;
pub use crown::{align_frond_direction, crown_directions, length_scale};
pub use spine::spine_at;

/// CLI / shape parameters for a frond crown.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FrondCrownShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 9))]
	pub frond_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.4))]
	pub length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.18))]
	pub width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub droop: f32,
	/// Mid-span arch along each frond before tip droop (0 = droop-only spine).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub arch_lift: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.35))]
	pub twist: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 36))]
	pub leaflet_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 20))]
	pub spine_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.022))]
	pub shoot_half_radius: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.008))]
	pub rachis_half_thickness: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 3.2))]
	pub leaflet_length_scale: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.62))]
	pub downward_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.48))]
	pub outward_spread_radians: f32,
	/// Initial emission pitch above horizontal (pairs with [`Self::arch_lift`] for up-and-over palms).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub emission_lift_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for FrondCrownShape {
	fn default() -> Self {
		Self {
			frond_count: 9,
			length: 1.4,
			width: 0.18,
			droop: 0.55,
			arch_lift: 0.0,
			twist: 0.35,
			leaflet_count: 36,
			spine_segments: 20,
			shoot_half_radius: 0.022,
			rachis_half_thickness: 0.008,
			leaflet_length_scale: 3.2,
			downward_tilt_radians: 0.62,
			outward_spread_radians: 0.48,
			emission_lift_radians: 0.0,
			seed: 0,
		}
	}
}

/// One straight frond segment along a drooping rachis (VegetationComponents GLB emission).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrondRachisSegment {
	pub start: Vec3,
	pub direction: Vec3,
	pub length: f32,
	pub width: f32,
}

impl FrondCrownShape {
	pub fn frond_config(&self, scale: f32) -> FrondConfig {
		FrondConfig {
			segments: self.spine_segments.max(1),
			length: (self.length * scale).max(1e-4),
			width: (self.width * scale).max(1e-6),
			droop: self.droop * scale,
			arch_lift: self.arch_lift * scale,
			twist: self.twist,
			leaflet_count: self.leaflet_count.max(2),
		}
	}

	/// Connected rachis runs (one chain per frond) for VegetationComponents at `origin`.
	///
	/// Samples the same droop / arch spine as the procedural mesh, then emits straight
	/// frond-segment chords along that polyline (kit +Y along each chord).
	pub fn frond_runs_at(&self, origin: Vec3) -> Vec<Vec<FrondRachisSegment>> {
		let config = self.frond_config(1.0);
		let rings = self.spine_segments.max(1) as usize;
		let width = config.width.max(1e-6);
		let directions = crown_directions(
			self.frond_count,
			self.seed,
			self.downward_tilt_radians,
			self.outward_spread_radians,
			self.emission_lift_radians,
		);

		let mut runs = Vec::with_capacity(directions.len());
		for (i, direction) in directions.into_iter().enumerate() {
			let mut element_config = config;
			element_config.length *= length_scale(i as u32, self.seed, 0.82, 1.08);
			let rotation = align_frond_direction(direction);
			let mut points = Vec::with_capacity(rings + 1);
			for ring in 0..=rings {
				let t = ring as f32 / rings as f32;
				points.push(origin + rotation * spine_at(&element_config, t));
			}

			let mut run = Vec::with_capacity(rings);
			for seg in 0..rings {
				let start = points[seg];
				let end = points[seg + 1];
				let ray = end - start;
				let length = ray.length();
				if length < 1e-5 {
					continue;
				}
				run.push(FrondRachisSegment { start, direction: ray / length, length, width });
			}
			if !run.is_empty() {
				runs.push(run);
			}
		}
		runs
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn crown_defaults_droop_outward() {
		for d in crown_directions(
			FrondCrownShape::default().frond_count,
			0,
			FrondCrownShape::default().downward_tilt_radians,
			FrondCrownShape::default().outward_spread_radians,
			FrondCrownShape::default().emission_lift_radians,
		) {
			assert!(d.y < 0.0, "palm fronds should droop downward: {d:?}");
		}
	}

	#[test]
	fn rachis_tips_hang_below_a_straight_emission() {
		let shape =
			FrondCrownShape { frond_count: 10, spine_segments: 8, seed: 3, ..Default::default() };
		for run in shape.frond_runs_at(Vec3::ZERO) {
			let first = run.first().expect("segment");
			let last = run.last().expect("segment");
			let tip = last.start + last.direction * last.length;
			let chain_len: f32 = run.iter().map(|s| s.length).sum();
			let straight_tip_y = first.start.y + first.direction.y * chain_len;
			assert!(
				tip.y < straight_tip_y - 0.02,
				"blade stayed straight out: tip.y={} straight.y={} first={:?}",
				tip.y,
				straight_tip_y,
				first.direction
			);
		}
	}
}
