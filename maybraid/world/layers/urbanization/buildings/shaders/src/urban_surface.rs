//! Urban surface [`Material`] — [`MaterialRef`] bags plus a named recipe kind.
//!
//! The fragment shader adds implicit-surface POM (Durham terrain solver)
//! against a per-kind height field in the dominant face plane.

use bevy::{
	asset::embedded_asset,
	prelude::*,
	reflect::TypePath,
	render::render_resource::{AsBindGroup, ShaderType},
	shader::ShaderRef,
};
use material_ref::{
	MaterialId, MaterialRasters, MaterialRef, MATERIAL_PALETTE_SLOTS, MATERIAL_RASTER_CHANNELS,
	MATERIAL_RASTER_WIDTH, MATERIAL_SCALAR_FLOATS,
};

pub const RECIPE_STUCCO: &str = "stucco";
pub const RECIPE_TERRACOTTA: &str = "terracotta";
pub const RECIPE_WOOD: &str = "wood";
pub const RECIPE_HAY: &str = "hay";
pub const RECIPE_IRON: &str = "iron";
pub const RECIPE_STONE: &str = "stone";

pub const KIND_STUCCO: u32 = 0;
pub const KIND_TERRACOTTA: u32 = 1;
pub const KIND_WOOD: u32 = 2;
pub const KIND_HAY: u32 = 3;
pub const KIND_IRON: u32 = 4;
pub const KIND_STONE: u32 = 5;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_STUCCO_COLOR: Vec4 = Vec4::new(0.78, 0.72, 0.62, 1.0);

/// Registers embedded **`urban_surface.wgsl`** and [`MaterialPlugin`].
pub struct UrbanSurfaceMaterialPlugin;

impl Plugin for UrbanSurfaceMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "urban_surface.wgsl");
		app.add_plugins(MaterialPlugin::<UrbanSurfaceMaterial>::default());
	}
}

/// Look index packed into [`UrbanSurfaceUniform::kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrbanSurfaceKind {
	Stucco,
	Terracotta,
	Wood,
	Hay,
	Iron,
	/// Coursed ashlar blocks with recessed mortar joints.
	Stone,
}

impl UrbanSurfaceKind {
	pub const fn as_u32(self) -> u32 {
		match self {
			Self::Stucco => KIND_STUCCO,
			Self::Terracotta => KIND_TERRACOTTA,
			Self::Wood => KIND_WOOD,
			Self::Hay => KIND_HAY,
			Self::Iron => KIND_IRON,
			Self::Stone => KIND_STONE,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		match name {
			RECIPE_TERRACOTTA => Self::Terracotta,
			RECIPE_WOOD => Self::Wood,
			RECIPE_HAY => Self::Hay,
			RECIPE_IRON => Self::Iron,
			RECIPE_STONE => Self::Stone,
			_ => Self::Stucco,
		}
	}
}

/// Packed GPU representation of one urban-surface [`MaterialRef`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct UrbanSurfaceUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub _pad: UVec3,
}

impl UrbanSurfaceUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let kind = match &material_ref.name {
			MaterialId::Name(name) => UrbanSurfaceKind::from_recipe_name(name),
			MaterialId::Default => UrbanSurfaceKind::Stucco,
		};

		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			colors[0] = DEFAULT_STUCCO_COLOR;
			colors[1] = DEFAULT_STUCCO_COLOR * Vec4::new(0.82, 0.78, 0.7, 1.0);
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
		for channel in 0..MATERIAL_RASTER_CHANNELS {
			let samples = material_ref.rasters.get_or(channel, 0.0);
			let rows = MaterialRasters::packed_rows(samples);
			rasters[channel] = rows.map(Vec4::from_array);
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
			kind: kind.as_u32(),
			_pad: UVec3::ZERO,
		}
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct UrbanSurfaceMaterial {
	#[uniform(0)]
	pub params: UrbanSurfaceUniform,
}

impl UrbanSurfaceMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: UrbanSurfaceUniform::from_material_ref(material_ref) }
	}
}

impl Default for UrbanSurfaceMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_STUCCO))
	}
}

impl Material for UrbanSurfaceMaterial {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "urban_surface.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

/// Named recipes claimed by [`crate::UrbanMaterialLib`].
pub fn is_urban_surface_recipe(name: &str) -> bool {
	matches!(
		name,
		RECIPE_STUCCO | RECIPE_TERRACOTTA | RECIPE_WOOD | RECIPE_HAY | RECIPE_IRON | RECIPE_STONE
	)
}

#[cfg(test)]
mod tests {
	use material_ref::MATERIAL_RASTER_SAMPLES;

	use super::*;

	#[test]
	fn from_material_ref_packs_palette_scalars_kind() {
		let material_ref = MaterialRef::named(RECIPE_IRON)
			.with_palette([Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)])
			.with_scalars([0.25, 0.5, 0.75])
			.with_raster(0, [1.0; MATERIAL_RASTER_SAMPLES]);
		let material = UrbanSurfaceMaterial::from_material_ref(&material_ref);
		assert_eq!(material.params.kind, KIND_IRON);
		assert!((material.params.colors[0].x - 1.0).abs() < 1e-5);
		assert_eq!(material.params.scalars[0], Vec4::new(0.25, 0.5, 0.75, 0.0));
		assert_eq!(material.params.rasters[0][0], Vec4::new(1.0, 1.0, 1.0, 0.0));
		assert!(is_urban_surface_recipe(RECIPE_STUCCO));
		assert!(!is_urban_surface_recipe("clothing_cloth"));
	}

	#[test]
	fn stone_recipe_packs_its_own_kind() {
		let material = UrbanSurfaceMaterial::from_material_ref(&MaterialRef::named(RECIPE_STONE));
		assert_eq!(material.params.kind, KIND_STONE);
		assert!(is_urban_surface_recipe(RECIPE_STONE));
	}
}
