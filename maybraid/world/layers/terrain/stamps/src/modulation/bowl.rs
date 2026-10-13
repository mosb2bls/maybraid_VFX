//! Softmask radial bowl: deeper toward the region center, with optional bed noise.

use crate::region::{Region2D, RegionNoise};
use bevy_math::Vec2;

/// Absolute bed carve inside a region: deeper toward the centroid, soft shore fade.
///
/// Bed target before clamping:
/// \[
/// b = \mathrm{lerp}(B_{\mathrm{shore}}, B_{\mathrm{center}}, (1-r)^p) + N
/// \]
/// where \(r\) is the region's normalized radial coordinate (0 at center, 1 on the
/// geometric shore). Bipolar [`Self::bed_noise`] may raise \(b\) above the water
/// surface for islands / peninsulas; [`Self::bed_ceiling`] caps that lift.
#[derive(Debug, Clone)]
pub struct RegionBowlModulation {
	pub region: Region2D,
	/// Absolute bed elevation at the centroid (deepest before noise).
	pub center_bed: f32,
	/// Absolute bed elevation on the geometric shore (before noise).
	pub shore_bed: f32,
	/// Max absolute bed elevation (island / peninsula ceiling).
	pub bed_ceiling: f32,
	/// Exponent on `(1 - r)` for the depth falloff (`1` = linear).
	pub falloff_power: f32,
	/// Softmask fade past the shore (world units along the SDF).
	pub outer_radius: f32,
	pub boundary_noise: Option<RegionNoise>,
	pub bed_noise: Option<RegionNoise>,
}

impl RegionBowlModulation {
	pub fn new(
		region: Region2D,
		center_bed: f32,
		shore_bed: f32,
		bed_ceiling: f32,
		falloff_power: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			center_bed,
			shore_bed,
			bed_ceiling,
			falloff_power: falloff_power.max(0.25),
			outer_radius: outer_radius.max(0.001),
			boundary_noise: None,
			bed_noise: None,
		}
	}

	pub fn with_boundary_noise(mut self, noise: RegionNoise) -> Self {
		self.boundary_noise = Some(noise);
		self
	}

	pub fn with_bed_noise(mut self, noise: RegionNoise) -> Self {
		self.bed_noise = Some(noise);
		self
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		let w =
			self.region
				.softmask_weight(p, 0.0, self.outer_radius, self.boundary_noise.as_ref());
		if w >= 1.0 {
			return elevation;
		}

		let r = self.region.radial_norm(p).clamp(0.0, 1.5);
		let t = (1.0 - r).clamp(0.0, 1.0).powf(self.falloff_power);
		let mut bed = self.shore_bed + (self.center_bed - self.shore_bed) * t;
		if let Some(hn) = &self.bed_noise {
			bed += hn.sample_height(p);
		}
		// Allow a little above the authored ceiling for noise overshoot, but keep
		// islands from punching through the surrounding shelf.
		bed = bed.min(self.bed_ceiling);

		elevation + (1.0 - w) * (bed - elevation)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::region::CircleRegion;

	#[test]
	fn bowl_deeper_at_center_than_mid() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 40.0 });
		let bowl = RegionBowlModulation::new(region, 20.0, 40.0, 45.0, 1.25, 2.0);
		let h_c = bowl.modify_elevation(45.0, 0.0, 0.0);
		let h_m = bowl.modify_elevation(45.0, 28.0, 0.0);
		assert!(h_c < h_m - 2.0, "center {h_c} should sit deeper than mid {h_m}");
		assert!((h_c - 20.0).abs() < 0.5, "center should approach center_bed, got {h_c}");
		Ok(())
	}

	#[test]
	fn bed_noise_can_raise_above_shore() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 40.0 });
		let noise = RegionNoise::from_seed(9, 0.05, 12.0);
		let bowl =
			RegionBowlModulation::new(region, 30.0, 40.0, 52.0, 1.0, 2.0).with_bed_noise(noise);
		let mut raised = false;
		for i in 0..48 {
			let ang = i as f32 * std::f32::consts::TAU / 48.0;
			// Near-shore ring: base bed is close to shore_bed so amp can crest above W.
			let p = Vec2::new(ang.cos(), ang.sin()) * 34.0;
			let h = bowl.modify_elevation(45.0, p.x, p.y);
			if h > 40.25 {
				raised = true;
				break;
			}
		}
		assert!(raised, "bipolar bed noise should lift some samples above shore_bed");
		Ok(())
	}
}
