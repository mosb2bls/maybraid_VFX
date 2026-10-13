//! Authored grass coverage and lush/dry blade palette.

use bevy::{prelude::*, render::render_resource::ShaderType};

/// GPU grass controls packed for [`super::TerrainShader`] binding 3.
///
/// **`params.x`** = authored coverage (`0` disables grass). Moisture noise
/// is local variation only; it is multiplied by this value so barren
/// ground cannot become meadow on its own.
///
/// **`params.y`** = how much [`super::TerrainShader::base_color`] tints
/// grass (`0` = palette only).
///
/// **`params.z`** = travelling wind-color amount. Keep this small so
/// bending stays the primary motion.
///
/// **`params.w`** reserved.
///
/// Palette fields are linear RGB in **xyz**; **w** unused.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainGrassUniform {
	pub params: Vec4,
	pub lush_root: Vec4,
	pub lush_mid: Vec4,
	pub lush_tip: Vec4,
	pub dry_root: Vec4,
	pub dry_mid: Vec4,
	pub dry_tip: Vec4,
}

impl TerrainGrassUniform {
	/// Vibrant defaults from the POM-grass trial: dense green turf, dry
	/// yellow only where moisture noise is low.
	pub fn vibrant() -> Self {
		Self {
			params: Vec4::new(1.0, 0.15, 0.035, 0.0),
			lush_root: rgb(0.045, 0.23, 0.018),
			lush_mid: rgb(0.13, 0.48, 0.032),
			lush_tip: rgb(0.38, 0.70, 0.085),
			dry_root: rgb(0.13, 0.16, 0.025),
			dry_mid: rgb(0.32, 0.38, 0.075),
			dry_tip: rgb(0.62, 0.62, 0.18),
		}
	}

	/// Same coverage, but both moisture ends use the dry straw palette.
	pub fn dry() -> Self {
		let straw = Self::vibrant();
		Self {
			params: straw.params,
			lush_root: straw.dry_root,
			lush_mid: straw.dry_mid,
			lush_tip: straw.dry_tip,
			dry_root: straw.dry_root,
			dry_mid: straw.dry_mid,
			dry_tip: straw.dry_tip,
		}
	}

	/// Desaturated olive / tan. Coverage stays on so the mat is still there.
	pub fn muted() -> Self {
		Self {
			params: Vec4::new(1.0, 0.20, 0.025, 0.0),
			lush_root: rgb(0.10, 0.14, 0.06),
			lush_mid: rgb(0.22, 0.28, 0.12),
			lush_tip: rgb(0.40, 0.42, 0.22),
			dry_root: rgb(0.16, 0.15, 0.08),
			dry_mid: rgb(0.30, 0.28, 0.16),
			dry_tip: rgb(0.48, 0.44, 0.26),
		}
	}

	/// Coverage zero: shader skips turf, blades, and the crack-cover gate.
	pub fn disabled() -> Self {
		Self::vibrant().with_coverage(0.0)
	}

	#[inline]
	pub fn coverage(&self) -> f32 {
		self.params.x
	}

	#[inline]
	pub fn tint_amount(&self) -> f32 {
		self.params.y
	}

	#[inline]
	pub fn wind_color(&self) -> f32 {
		self.params.z
	}

	#[inline]
	pub fn with_coverage(mut self, coverage: f32) -> Self {
		self.params.x = coverage;
		self
	}

	#[inline]
	pub fn with_tint_amount(mut self, tint_amount: f32) -> Self {
		self.params.y = tint_amount;
		self
	}

	#[inline]
	pub fn with_wind_color(mut self, wind_color: f32) -> Self {
		self.params.z = wind_color;
		self
	}
}

impl Default for TerrainGrassUniform {
	fn default() -> Self {
		Self::vibrant()
	}
}

#[inline]
fn rgb(r: f32, g: f32, b: f32) -> Vec4 {
	Vec4::new(r, g, b, 0.0)
}
