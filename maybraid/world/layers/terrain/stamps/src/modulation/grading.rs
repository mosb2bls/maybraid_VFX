//! Softmask grading height operator between two endpoints.

use crate::region::{Region2D, RegionNoise};
use bevy_math::Vec2;

/// Blend toward a graded elevation between two endpoints inside a region.
#[derive(Debug, Clone)]
pub struct RegionGradingModulation {
	pub region: Region2D,
	pub start: Vec2,
	pub start_elevation: f32,
	pub end: Vec2,
	pub end_elevation: f32,
	pub noise: Option<RegionNoise>,
	pub inner_radius: f32,
	pub outer_radius: f32,
	/// When set, never raise above the incoming elevation (cut / floor bias only).
	pub depression_only: bool,
}

impl RegionGradingModulation {
	pub fn new(
		region: Region2D,
		start: Vec2,
		start_elevation: f32,
		end: Vec2,
		end_elevation: f32,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			start,
			start_elevation,
			end,
			end_elevation,
			noise: None,
			inner_radius,
			outer_radius: outer_radius.max(inner_radius + 0.001),
			depression_only: false,
		}
	}

	pub fn with_noise(mut self, noise: RegionNoise) -> Self {
		self.noise = Some(noise);
		self
	}

	/// Cut toward the grade floor; leave natural lows untouched.
	pub fn depression_only(mut self) -> Self {
		self.depression_only = true;
		self
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		let distance_to_start = (p - self.start).length();
		let distance_to_end = (p - self.end).length();
		let denom = (distance_to_start + distance_to_end).max(1e-3);
		let progress = distance_to_start / denom;
		let graded = self.start_elevation + (self.end_elevation - self.start_elevation) * progress;
		let weight = self.region.softmask_weight(
			p,
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		);
		let toward = weight * elevation + (1.0 - weight) * graded;
		if self.depression_only {
			toward.min(elevation)
		} else {
			toward
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::region::CircleRegion;

	#[test]
	fn depression_only_never_raises_natural_lows() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 20.0 });
		let g = RegionGradingModulation::new(
			region,
			Vec2::new(-10.0, 0.0),
			50.0,
			Vec2::new(10.0, 0.0),
			40.0,
			5.0,
			15.0,
		)
		.depression_only();
		// Natural valley below the baked grade — must not lift.
		assert_eq!(g.modify_elevation(30.0, 0.0, 0.0), 30.0);
		// High ground inside the corridor — may cut down.
		assert!(g.modify_elevation(80.0, 0.0, 0.0) < 80.0);
		Ok(())
	}
}
