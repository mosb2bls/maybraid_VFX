//! Doric column-shaft [`Material`] — [`MaterialRef`] bags plus UV POM fluting.
//!
//! Relief is color-only (cavity + bent normal). The shaft outline, capital,
//! pedestal, and cover collision stay real geometry. See `COLUMN_SURFACE.md`
//! and [`shaft::placeholder_shaft_mesh`] for the UV / tangent contract the
//! final column GLB must meet.

use bevy::{
	asset::embedded_asset,
	prelude::*,
	reflect::TypePath,
	render::render_resource::{AsBindGroup, ShaderType},
	shader::ShaderRef,
};
use material_ref::{
	MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS, MATERIAL_RASTER_WIDTH,
	MATERIAL_SCALAR_FLOATS, MaterialRasters, MaterialRef,
};

pub mod shaft;

pub const RECIPE_DORIC_COLUMN: &str = "doric_column";
pub const KIND_DORIC: u32 = 0;

/// Classical Doric flute count. Overridable via [`MaterialRef`] scalar 3.
pub const DEFAULT_FLUTE_COUNT: f32 = 20.0;
/// Default POM fade start in metres. Overridable via scalar 4.
pub const DEFAULT_POM_FADE_START_M: f32 = 28.0;
/// Default POM fade end in metres. Overridable via scalar 5.
pub const DEFAULT_POM_FADE_END_M: f32 = 85.0;
/// Hard cap on the implicit-surface march. Overridable via scalar 6, clamped.
pub const DEFAULT_POM_MAX_STEPS: f32 = 48.0;
/// Default flute relief depth in metres. Overridable via scalar 7.
pub const DEFAULT_RELIEF_DEPTH_M: f32 = 0.055;
/// Scalar 8: `>= 0.5` skips POM and flute cavity (flat limestone A/B).
pub const SCALAR_FLAT: usize = 8;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_LIMESTONE: Vec4 = Vec4::new(0.84, 0.80, 0.72, 1.0);
const DEFAULT_ACCENT: Vec4 = Vec4::new(0.58, 0.52, 0.44, 1.0);

/// Registers embedded **`column_surface.wgsl`** and [`MaterialPlugin`].
pub struct ColumnSurfaceMaterialPlugin;

impl Plugin for ColumnSurfaceMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "column_surface.wgsl");
		app.add_plugins(MaterialPlugin::<ColumnSurfaceMaterial>::default());
	}
}

/// Packed GPU representation of one column-surface [`MaterialRef`].
///
/// Scalar pad (same layout the fragment shader reads):
/// - `0` roughness override (`0` keeps the look)
/// - `1` grain scale
/// - `2` wear / lichen in flute floors
/// - `3` flute count (`0` → [`DEFAULT_FLUTE_COUNT`])
/// - `4` POM fade start metres (`0` → [`DEFAULT_POM_FADE_START_M`])
/// - `5` POM fade end metres (`0` → [`DEFAULT_POM_FADE_END_M`])
/// - `6` max march steps (`0` → [`DEFAULT_POM_MAX_STEPS`])
/// - `7` relief depth metres (`0` → [`DEFAULT_RELIEF_DEPTH_M`])
/// - `8` flat / POM-off (`>= 0.5` skips march and flute cavity)
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct ColumnSurfaceUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub _pad: UVec3,
}

impl ColumnSurfaceUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			colors[0] = DEFAULT_LIMESTONE;
			colors[1] = DEFAULT_ACCENT;
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
			}
			let first = colors[0];
			for color in colors.iter_mut().skip(material_ref.palette.len()) {
				*color = first;
			}
		}

		let mut scalars = [Vec4::ZERO; SCALAR_VEC4S];
		let values = material_ref.scalar_values();
		for (i, slot) in scalars.iter_mut().enumerate() {
			let base = i * 4;
			*slot = Vec4::new(
				values.get(base).copied().unwrap_or(0.0),
				values.get(base + 1).copied().unwrap_or(0.0),
				values.get(base + 2).copied().unwrap_or(0.0),
				values.get(base + 3).copied().unwrap_or(0.0),
			);
		}

		let mut rasters = [[Vec4::ZERO; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS];
		for (channel, raster) in rasters.iter_mut().enumerate() {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			*raster = rows.map(Vec4::from_array);
		}

		Self {
			colors,
			noise: Vec4::new(
				material_ref.noise.frequency.max(1e-6),
				material_ref.noise.amplitude,
				material_ref.noise.seed as f32,
				material_ref.noise.octaves as f32,
			),
			scalars,
			rasters,
			kind: KIND_DORIC,
			_pad: UVec3::ZERO,
		}
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ColumnSurfaceMaterial {
	#[uniform(0)]
	pub params: ColumnSurfaceUniform,
}

impl ColumnSurfaceMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: ColumnSurfaceUniform::from_material_ref(material_ref) }
	}
}

impl Default for ColumnSurfaceMaterial {
	fn default() -> Self {
		Self::from_material_ref(&doric_column_ref())
	}
}

impl Material for ColumnSurfaceMaterial {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "column_surface.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

/// Named recipes claimed by the column-surface lib.
pub fn is_column_surface_recipe(name: &str) -> bool {
	matches!(name, RECIPE_DORIC_COLUMN | "doric")
}

/// Authored limestone Doric look used by the materials playground and kits.
pub fn doric_column_ref() -> MaterialRef {
	MaterialRef::named(RECIPE_DORIC_COLUMN)
		.with_palette([
			Color::srgb(0.84, 0.80, 0.72),
			Color::srgb(0.58, 0.52, 0.44),
			Color::srgb(0.46, 0.50, 0.38),
		])
		.with_scalars([0.0, 0.35, 0.22])
}

/// Same limestone recipe with POM and flute cavity disabled (review A/B).
pub fn doric_column_flat_ref() -> MaterialRef {
	MaterialRef::named(RECIPE_DORIC_COLUMN)
		.with_palette([
			Color::srgb(0.84, 0.80, 0.72),
			Color::srgb(0.58, 0.52, 0.44),
			Color::srgb(0.46, 0.50, 0.38),
		])
		.with_scalars([0.0, 0.35, 0.22, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0])
}

#[cfg(test)]
mod tests {
	use material_ref::{MATERIAL_RASTER_SAMPLES, MaterialId};

	use super::*;

	#[test]
	fn from_material_ref_packs_palette_scalars_kind() -> anyhow::Result<()> {
		let material_ref = MaterialRef::named(RECIPE_DORIC_COLUMN)
			.with_palette([Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)])
			.with_scalars([0.25, 0.5, 0.75, 16.0, 20.0, 70.0, 24.0, 0.05])
			.with_raster(0, [1.0; MATERIAL_RASTER_SAMPLES]);
		let material = ColumnSurfaceMaterial::from_material_ref(&material_ref);
		assert_eq!(material.params.kind, KIND_DORIC);
		assert!((material.params.colors[0].x - 1.0).abs() < 1e-5);
		assert_eq!(material.params.scalars[0], Vec4::new(0.25, 0.5, 0.75, 16.0));
		assert_eq!(material.params.scalars[1], Vec4::new(20.0, 70.0, 24.0, 0.05));
		assert_eq!(material.params.rasters[0][0], Vec4::new(1.0, 1.0, 1.0, 0.0));
		assert!(is_column_surface_recipe(RECIPE_DORIC_COLUMN));
		assert!(is_column_surface_recipe("doric"));
		assert!(!is_column_surface_recipe("stone"));
		Ok(())
	}

	#[test]
	fn empty_palette_uses_limestone_defaults() -> anyhow::Result<()> {
		let material =
			ColumnSurfaceMaterial::from_material_ref(&MaterialRef::named(RECIPE_DORIC_COLUMN));
		assert_eq!(material.params.colors[0], DEFAULT_LIMESTONE);
		assert_eq!(material.params.colors[1], DEFAULT_ACCENT);
		assert_eq!(material.params.kind, KIND_DORIC);
		Ok(())
	}

	#[test]
	fn doric_column_ref_keeps_the_recipe_name() -> anyhow::Result<()> {
		let material_ref = doric_column_ref();
		assert_eq!(material_ref.name, MaterialId::named(RECIPE_DORIC_COLUMN));
		assert_eq!(material_ref.scalar_values(), [0.0, 0.35, 0.22]);
		Ok(())
	}

	#[test]
	fn flat_ref_sets_the_pom_off_scalar() -> anyhow::Result<()> {
		let material = ColumnSurfaceMaterial::from_material_ref(&doric_column_flat_ref());
		assert!((material.params.scalars[2].x - 1.0).abs() < 1e-5);
		assert_eq!(SCALAR_FLAT, 8);
		Ok(())
	}

	#[test]
	fn quality_defaults_are_the_documented_bounds() -> anyhow::Result<()> {
		assert_eq!(DEFAULT_FLUTE_COUNT, 20.0);
		assert_eq!(DEFAULT_POM_FADE_START_M, 28.0);
		assert_eq!(DEFAULT_POM_FADE_END_M, 85.0);
		assert_eq!(DEFAULT_POM_MAX_STEPS, 48.0);
		assert_eq!(DEFAULT_RELIEF_DEPTH_M, 0.055);
		assert!(DEFAULT_POM_FADE_END_M > DEFAULT_POM_FADE_START_M);
		Ok(())
	}
}
