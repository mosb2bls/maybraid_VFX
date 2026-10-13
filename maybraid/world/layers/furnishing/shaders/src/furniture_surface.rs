//! Furniture surface [`Material`] — [`MaterialRef`] bags plus a named recipe kind.

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

pub const RECIPE_FURNITURE_WOOD: &str = "furniture_wood";
pub const RECIPE_FURNITURE_CLOTH: &str = "furniture_cloth";
pub const RECIPE_FURNITURE_SOFT: &str = "furniture_soft";
pub const RECIPE_FURNITURE_MARBLE: &str = "furniture_marble";
pub const RECIPE_FURNITURE_ORNATE: &str = "furniture_ornate";
pub const RECIPE_FURNITURE_LAVA: &str = "furniture_lava";
pub const RECIPE_FURNITURE_COSMOS: &str = "furniture_cosmos";
pub const RECIPE_FURNITURE_SCALES: &str = "furniture_scales";
pub const RECIPE_FURNITURE_LACQUER: &str = "furniture_lacquer";
pub const RECIPE_FURNITURE_METAL: &str = "furniture_metal";
pub const RECIPE_FURNITURE_ROCKADDER: &str = "furniture_rockadder";

pub const KIND_WOOD: u32 = 0;
pub const KIND_CLOTH: u32 = 1;
pub const KIND_SOFT: u32 = 2;
pub const KIND_MARBLE: u32 = 3;
pub const KIND_ORNATE: u32 = 4;
pub const KIND_LAVA: u32 = 5;
pub const KIND_COSMOS: u32 = 6;
pub const KIND_SCALES: u32 = 7;
pub const KIND_LACQUER: u32 = 8;
pub const KIND_METAL: u32 = 9;
pub const KIND_ROCKADDER: u32 = 10;

const SCALAR_VEC4S: usize = MATERIAL_SCALAR_FLOATS / 4;
const DEFAULT_WOOD: Vec4 = Vec4::new(0.72, 0.46, 0.20, 1.0);

/// Registers embedded **`furniture_surface.wgsl`** and [`MaterialPlugin`].
pub struct FurnitureSurfaceMaterialPlugin;

impl Plugin for FurnitureSurfaceMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "furniture_surface.wgsl");
		app.add_plugins(MaterialPlugin::<FurnitureSurfaceMaterial>::default());
	}
}

/// Look index packed into [`FurnitureSurfaceUniform::kind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FurnitureSurfaceKind {
	Wood,
	Cloth,
	Soft,
	Marble,
	Ornate,
	Lava,
	Cosmos,
	Scales,
	Lacquer,
	Metal,
	Rockadder,
}

impl FurnitureSurfaceKind {
	pub const fn as_u32(self) -> u32 {
		match self {
			Self::Wood => KIND_WOOD,
			Self::Cloth => KIND_CLOTH,
			Self::Soft => KIND_SOFT,
			Self::Marble => KIND_MARBLE,
			Self::Ornate => KIND_ORNATE,
			Self::Lava => KIND_LAVA,
			Self::Cosmos => KIND_COSMOS,
			Self::Scales => KIND_SCALES,
			Self::Lacquer => KIND_LACQUER,
			Self::Metal => KIND_METAL,
			Self::Rockadder => KIND_ROCKADDER,
		}
	}

	pub fn from_recipe_name(name: &str) -> Self {
		match name {
			RECIPE_FURNITURE_CLOTH => Self::Cloth,
			RECIPE_FURNITURE_SOFT => Self::Soft,
			RECIPE_FURNITURE_MARBLE => Self::Marble,
			RECIPE_FURNITURE_ORNATE => Self::Ornate,
			RECIPE_FURNITURE_LAVA => Self::Lava,
			RECIPE_FURNITURE_COSMOS => Self::Cosmos,
			RECIPE_FURNITURE_SCALES => Self::Scales,
			RECIPE_FURNITURE_LACQUER => Self::Lacquer,
			RECIPE_FURNITURE_METAL => Self::Metal,
			RECIPE_FURNITURE_ROCKADDER => Self::Rockadder,
			_ => Self::Wood,
		}
	}
}

/// Packed GPU representation of one furniture-surface [`MaterialRef`].
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct FurnitureSurfaceUniform {
	pub colors: [Vec4; MATERIAL_PALETTE_SLOTS],
	/// `x` frequency, `y` amplitude, `z` seed, `w` octaves.
	pub noise: Vec4,
	pub scalars: [Vec4; SCALAR_VEC4S],
	pub rasters: [[Vec4; MATERIAL_RASTER_WIDTH]; MATERIAL_RASTER_CHANNELS],
	pub kind: u32,
	pub _pad: UVec3,
}

impl FurnitureSurfaceUniform {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		let kind = match &material_ref.name {
			MaterialId::Name(name) => FurnitureSurfaceKind::from_recipe_name(name),
			MaterialId::Default => FurnitureSurfaceKind::Wood,
		};

		let mut colors = [Vec4::ZERO; MATERIAL_PALETTE_SLOTS];
		if material_ref.palette.is_empty() {
			colors[0] = DEFAULT_WOOD;
			colors[1] = Vec4::new(0.95, 0.70, 0.28, 1.0);
		} else {
			for (slot, color) in colors.iter_mut().zip(&material_ref.palette) {
				let linear = LinearRgba::from(*color);
				*slot = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
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
pub struct FurnitureSurfaceMaterial {
	#[uniform(0)]
	pub params: FurnitureSurfaceUniform,
}

impl FurnitureSurfaceMaterial {
	pub fn from_material_ref(material_ref: &MaterialRef) -> Self {
		Self { params: FurnitureSurfaceUniform::from_material_ref(material_ref) }
	}
}

impl Default for FurnitureSurfaceMaterial {
	fn default() -> Self {
		Self::from_material_ref(&MaterialRef::named(RECIPE_FURNITURE_WOOD))
	}
}

impl Material for FurnitureSurfaceMaterial {
	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "furniture_surface.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}
}

/// Named recipes claimed by [`crate::FurnitureMaterialLib`].
pub fn is_furniture_surface_recipe(name: &str) -> bool {
	matches!(
		name,
		RECIPE_FURNITURE_WOOD
			| RECIPE_FURNITURE_CLOTH
			| RECIPE_FURNITURE_SOFT
			| RECIPE_FURNITURE_MARBLE
			| RECIPE_FURNITURE_ORNATE
			| RECIPE_FURNITURE_LAVA
			| RECIPE_FURNITURE_COSMOS
			| RECIPE_FURNITURE_SCALES
			| RECIPE_FURNITURE_LACQUER
			| RECIPE_FURNITURE_METAL
			| RECIPE_FURNITURE_ROCKADDER
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_material_ref_packs_palette_and_kind() {
		let material_ref = MaterialRef::named(RECIPE_FURNITURE_MARBLE)
			.with_palette([Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)]);
		let material = FurnitureSurfaceMaterial::from_material_ref(&material_ref);
		assert_eq!(material.params.kind, KIND_MARBLE);
		assert!((material.params.colors[0].x - 1.0).abs() < 1e-5);
		assert!(is_furniture_surface_recipe(RECIPE_FURNITURE_ORNATE));
		assert!(!is_furniture_surface_recipe("wood"));
		let lava =
			FurnitureSurfaceMaterial::from_material_ref(&MaterialRef::named(RECIPE_FURNITURE_LAVA));
		assert_eq!(lava.params.kind, KIND_LAVA);
		let rock = FurnitureSurfaceMaterial::from_material_ref(&MaterialRef::named(
			RECIPE_FURNITURE_ROCKADDER,
		));
		assert_eq!(rock.params.kind, KIND_ROCKADDER);
	}
}
