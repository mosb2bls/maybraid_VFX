//! Rim band + add-only height-noise recipe (primitive).
//!
//! Drawn height noise lives on [`crate::primitive::parameters::HydroParams::rim_height`]
//! after leaf bake so recipe structs stay [`Copy`].

/// Raise-only bank band and rim-height noise recipe.
#[derive(Debug, Clone, Copy)]
pub struct RimParams {
	/// Berm width outside wet support (world units).
	pub width: f32,
	/// Raise above shelf / free surface before height noise.
	pub lift: f32,
	/// Absolute shelf / rim anchor (lakes). When set, bank ≈ `shelf_anchor + lift`.
	pub shelf_anchor: Option<f32>,
	/// Hard cap on drawn rim height contribution.
	pub uplift_cap: f32,
	/// Rim height-noise amplitude low (world units).
	pub height_amp_min: f32,
	/// Rim height-noise amplitude high (world units).
	pub height_amp_max: f32,
	/// Rim height-noise frequency low (at authored noise ref radius).
	pub height_freq_min: f32,
	/// Rim height-noise frequency high (at authored noise ref radius).
	pub height_freq_max: f32,
}

impl Default for RimParams {
	fn default() -> Self {
		Self {
			width: 4.0,
			lift: 1.1,
			shelf_anchor: None,
			uplift_cap: crate::primitive::parameters::DEFAULT_RIM_UPLIFT_CAP,
			height_amp_min: 15.0,
			height_amp_max: 120.0,
			height_freq_min: 0.005,
			height_freq_max: 0.012,
		}
	}
}

impl RimParams {
	/// Stronger, longer-wavelength berm (shared lake/stream leaf default).
	pub fn with_visible_rim_bank(mut self) -> Self {
		self.height_amp_min = 10.0;
		self.height_amp_max = 20.0;
		self.height_freq_min = 0.008;
		self.height_freq_max = 0.02;
		self
	}

	/// Cap used when baking drawn height from the amp recipe.
	pub fn recipe_uplift_cap(self) -> f32 {
		self.height_amp_max.max(self.height_amp_min).max(0.0)
	}
}
