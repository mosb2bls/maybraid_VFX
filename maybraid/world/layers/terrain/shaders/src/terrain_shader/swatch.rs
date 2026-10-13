//! Per-swatch GPU layout and default **8-slot** palette tables (one lithology / role per index).

use bevy::{prelude::*, render::render_resource::ShaderType};

/// Blend **`left.xyz` → `right.xyz`**; **`swatch_meta.x`** = fold-in weight (0–1). **`swatch_meta.yzw`** unused.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainSwatchUniform {
	pub left: Vec4,
	pub right: Vec4,
	pub swatch_meta: Vec4,
}

/// Shared fold weight for default palette ramps.
pub const EVEN_SWATCH_FOLD_WEIGHT: f32 = 1.0 / 8.0;

#[inline]
fn rgb(r: f32, g: f32, b: f32) -> Vec3 {
	Vec3::new(r, g, b)
}

#[inline]
fn w() -> f32 {
	EVEN_SWATCH_FOLD_WEIGHT
}

impl TerrainSwatchUniform {
	/// Linear ramp from `left_rgb` to `right_rgb` with shader fold weight `fold_weight`.
	pub fn transition(left_rgb: Vec3, right_rgb: Vec3, fold_weight: f32) -> Self {
		Self {
			left: left_rgb.extend(0.0),
			right: right_rgb.extend(0.0),
			swatch_meta: Vec4::new(fold_weight, 0.0, 0.0, 0.0),
		}
	}

	pub fn with_fold_weight(mut self, fold_weight: f32) -> Self {
		self.swatch_meta.x = fold_weight;
		self
	}

	pub fn with_left_rgb(mut self, rgb: Vec3) -> Self {
		self.left = rgb.extend(0.0);
		self
	}

	pub fn with_right_rgb(mut self, rgb: Vec3) -> Self {
		self.right = rgb.extend(0.0);
		self
	}

	// --- Band 0 (macro): coarse lithology ---

	pub fn macro_weathered_greige() -> Self {
		Self::transition(rgb(0.46, 0.38, 0.34), rgb(0.62, 0.53, 0.49), w())
	}
	pub fn macro_dune_gold() -> Self {
		Self::transition(rgb(0.74, 0.58, 0.34), rgb(0.92, 0.78, 0.48), w())
	}
	pub fn macro_baked_clay() -> Self {
		Self::transition(rgb(0.48, 0.20, 0.12), rgb(0.68, 0.34, 0.20), w())
	}
	pub fn macro_soft_chalk() -> Self {
		Self::transition(rgb(0.82, 0.80, 0.70), rgb(0.98, 0.96, 0.86), w())
	}
	pub fn macro_sage_alluvium() -> Self {
		Self::transition(rgb(0.46, 0.50, 0.36), rgb(0.66, 0.64, 0.42), w())
	}
	pub fn macro_rust_band() -> Self {
		Self::transition(rgb(0.58, 0.34, 0.12), rgb(0.82, 0.56, 0.22), w())
	}
	pub fn macro_shale_cool() -> Self {
		Self::transition(rgb(0.10, 0.12, 0.13), rgb(0.28, 0.30, 0.32), w())
	}
	pub fn macro_shadow_oxide() -> Self {
		Self::transition(rgb(0.07, 0.05, 0.04), rgb(0.34, 0.13, 0.08), w())
	}

	// --- Band 1 (meso): same roles, higher contrast ---

	pub fn meso_weathered_greige() -> Self {
		Self::transition(rgb(0.34, 0.28, 0.26), rgb(0.76, 0.66, 0.62), w())
	}
	pub fn meso_dune_gold() -> Self {
		Self::transition(rgb(0.56, 0.38, 0.16), rgb(0.98, 0.84, 0.46), w())
	}
	pub fn meso_baked_clay() -> Self {
		Self::transition(rgb(0.30, 0.10, 0.06), rgb(0.78, 0.32, 0.16), w())
	}
	pub fn meso_soft_chalk() -> Self {
		Self::transition(rgb(0.66, 0.64, 0.54), rgb(1.00, 0.98, 0.88), w())
	}
	pub fn meso_sage_alluvium() -> Self {
		Self::transition(rgb(0.30, 0.38, 0.26), rgb(0.76, 0.74, 0.46), w())
	}
	pub fn meso_rust_band() -> Self {
		Self::transition(rgb(0.42, 0.18, 0.05), rgb(0.95, 0.58, 0.18), w())
	}
	pub fn meso_shale_cool() -> Self {
		Self::transition(rgb(0.04, 0.06, 0.07), rgb(0.40, 0.43, 0.46), w())
	}
	pub fn meso_shadow_oxide() -> Self {
		Self::transition(rgb(0.03, 0.02, 0.02), rgb(0.55, 0.16, 0.08), w())
	}

	// --- Band 2 (finer): same roles, strongest natural span ---

	pub fn finer_weathered_greige() -> Self {
		Self::transition(rgb(0.20, 0.16, 0.15), rgb(0.88, 0.78, 0.72), w())
	}
	pub fn finer_dune_gold() -> Self {
		Self::transition(rgb(0.42, 0.25, 0.08), rgb(1.00, 0.90, 0.48), w())
	}
	pub fn finer_baked_clay() -> Self {
		Self::transition(rgb(0.18, 0.05, 0.03), rgb(0.92, 0.36, 0.14), w())
	}
	pub fn finer_soft_chalk() -> Self {
		Self::transition(rgb(0.50, 0.48, 0.40), rgb(1.00, 1.00, 0.92), w())
	}
	pub fn finer_sage_alluvium() -> Self {
		Self::transition(rgb(0.18, 0.28, 0.18), rgb(0.86, 0.82, 0.48), w())
	}
	pub fn finer_rust_band() -> Self {
		Self::transition(rgb(0.28, 0.08, 0.02), rgb(1.00, 0.64, 0.12), w())
	}
	pub fn finer_shale_cool() -> Self {
		Self::transition(rgb(0.01, 0.015, 0.02), rgb(0.52, 0.56, 0.60), w())
	}
	pub fn finer_shadow_oxide() -> Self {
		Self::transition(rgb(0.01, 0.005, 0.004), rgb(0.78, 0.18, 0.06), w())
	}

	// --- Band 3 (detail): bright mineral / accent chips ---

	pub fn detail_ivory_glint() -> Self {
		Self::transition(rgb(0.82, 0.80, 0.70), rgb(1.00, 0.98, 0.88), w())
	}
	pub fn detail_silver_mist() -> Self {
		Self::transition(rgb(0.18, 0.18, 0.19), rgb(0.82, 0.86, 0.90), w())
	}
	pub fn detail_gold_spark() -> Self {
		Self::transition(rgb(0.72, 0.46, 0.08), rgb(1.00, 0.88, 0.18), w())
	}
	pub fn detail_crimson_flash() -> Self {
		Self::transition(rgb(0.14, 0.02, 0.01), rgb(1.00, 0.24, 0.04), w())
	}
	pub fn detail_aqua_shard() -> Self {
		Self::transition(rgb(0.02, 0.45, 0.42), rgb(0.18, 0.95, 0.88), w())
	}
	pub fn detail_lime_chip() -> Self {
		Self::transition(rgb(0.04, 0.34, 0.12), rgb(0.36, 1.00, 0.30), w())
	}
	pub fn detail_rosy_spark() -> Self {
		Self::transition(rgb(0.44, 0.04, 0.12), rgb(1.00, 0.18, 0.42), w())
	}
	pub fn detail_violet_glare() -> Self {
		Self::transition(rgb(0.18, 0.08, 0.42), rgb(0.72, 0.42, 1.00), w())
	}

	/// Band 0 — coarsest: broad lithology buckets.
	pub fn palette_macro_scale() -> [TerrainSwatchUniform; 8] {
		[
			Self::macro_weathered_greige(),
			Self::macro_dune_gold(),
			Self::macro_baked_clay(),
			Self::macro_soft_chalk(),
			Self::macro_sage_alluvium(),
			Self::macro_rust_band(),
			Self::macro_shale_cool(),
			Self::macro_shadow_oxide(),
		]
	}

	/// Band 1 — meso: stronger separation, same rough roles.
	pub fn palette_meso_high_contrast() -> [TerrainSwatchUniform; 8] {
		[
			Self::meso_weathered_greige(),
			Self::meso_dune_gold(),
			Self::meso_baked_clay(),
			Self::meso_soft_chalk(),
			Self::meso_sage_alluvium(),
			Self::meso_rust_band(),
			Self::meso_shale_cool(),
			Self::meso_shadow_oxide(),
		]
	}

	/// Band 2 — finer: max natural contrast.
	pub fn palette_finer_high_contrast() -> [TerrainSwatchUniform; 8] {
		[
			Self::finer_weathered_greige(),
			Self::finer_dune_gold(),
			Self::finer_baked_clay(),
			Self::finer_soft_chalk(),
			Self::finer_sage_alluvium(),
			Self::finer_rust_band(),
			Self::finer_shale_cool(),
			Self::finer_shadow_oxide(),
		]
	}

	/// Band 3 — finest: mineral sparks / bright inclusions.
	pub fn palette_detail_fun() -> [TerrainSwatchUniform; 8] {
		[
			Self::detail_ivory_glint(),
			Self::detail_silver_mist(),
			Self::detail_gold_spark(),
			Self::detail_crimson_flash(),
			Self::detail_aqua_shard(),
			Self::detail_lime_chip(),
			Self::detail_rosy_spark(),
			Self::detail_violet_glare(),
		]
	}
}
