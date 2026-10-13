//! Softmask affine height operator: `a * elevation + b` inside a region.

use crate::region::{Region2D, RegionNoise};
use bevy_math::Vec2;

/// Softmask affine: `a * elevation + b` inside a region.
#[derive(Debug, Clone)]
pub struct RegionAffineModulation {
	pub region: Region2D,
	pub inner_scale: f32,
	pub inner_offset: f32,
	pub inner_radius: f32,
	pub outer_radius: f32,
	pub noise: Option<RegionNoise>,
	/// Optional vertical noise added to [`Self::inner_offset`] inside the softmask.
	pub height_noise: Option<RegionNoise>,
	/// When true, height noise only **adds** above the base offset (`+|sample|`).
	pub height_noise_add_only: bool,
}

impl RegionAffineModulation {
	pub fn new(
		region: Region2D,
		inner_scale: f32,
		inner_offset: f32,
		inner_radius: f32,
		outer_radius: f32,
	) -> Self {
		Self {
			region,
			inner_scale,
			inner_offset,
			inner_radius,
			outer_radius: outer_radius.max(inner_radius + 0.001),
			noise: None,
			height_noise: None,
			height_noise_add_only: false,
		}
	}

	pub fn with_noise(mut self, noise: RegionNoise) -> Self {
		self.noise = Some(noise);
		self
	}

	pub fn with_height_noise(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = false;
		self
	}

	/// Height noise that only raises above [`Self::inner_offset`] (never lowers).
	pub fn with_height_noise_add_only(mut self, noise: RegionNoise) -> Self {
		self.height_noise = Some(noise);
		self.height_noise_add_only = true;
		self
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		let w = self.region.softmask_weight(
			p,
			self.inner_radius,
			self.outer_radius,
			self.noise.as_ref(),
		);
		let mut offset = self.inner_offset;
		if let Some(hn) = &self.height_noise {
			let s = hn.sample_height(p);
			offset += if self.height_noise_add_only { s.abs() } else { s };
		}
		let a = self.inner_scale + (1.0 - self.inner_scale) * w;
		let b = offset * (1.0 - w);
		a * elevation + b
	}
}
