//! Apron band + indent-noise recipe (primitive).
//!
//! [`ApronParams::width`] is the blend band beyond the rim. Indent / frequency
//! ranges are drawn at leaf bake to expand that width (and related shore noise).

/// Outer grade band and apron-boundary noise recipe.
#[derive(Debug, Clone, Copy)]
pub struct ApronParams {
	/// Outer grade width beyond the rim band (world units).
	pub width: f32,
	/// Power for noise frequency scaling: `f ∝ (ref / radius)^power`.
	pub noise_freq_power: f32,
	/// Apron boundary indent as a fraction of apron width (low).
	pub indent_frac_min: f32,
	/// Apron boundary indent as a fraction of apron width (high).
	pub indent_frac_max: f32,
	/// Apron boundary frequency low (at authored noise ref radius).
	pub freq_min: f32,
	/// Apron boundary frequency high (at authored noise ref radius).
	pub freq_max: f32,
}

impl Default for ApronParams {
	fn default() -> Self {
		Self {
			width: 8.0,
			noise_freq_power: 0.5,
			indent_frac_min: 0.12,
			indent_frac_max: 0.40,
			freq_min: 0.005,
			freq_max: 0.012,
		}
	}
}
