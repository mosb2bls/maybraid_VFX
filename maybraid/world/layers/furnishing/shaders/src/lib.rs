//! Furniture shaders: named [`MaterialRef`] surface recipes.
//!
//! Parallel to Chico vegetation-shaders / Richmond urban-surface: one PBR
//! look graph keyed by recipe (`furniture_wood`, `furniture_lacquer`,
//! `furniture_metal`, `furniture_marble`, `furniture_ornate`, `furniture_lava`,
//! `furniture_cosmos`, `furniture_scales`, `furniture_rockadder`, …).
//! Mosaic, veins, nebula, and gold inlay are procedural. Wood is a BotW
//! painted stain with hue islands, not ring grain.

use bevy::prelude::*;

mod furniture_surface;
mod material_lib;

pub use furniture_surface::{
	is_furniture_surface_recipe, FurnitureSurfaceKind, FurnitureSurfaceMaterial,
	FurnitureSurfaceMaterialPlugin, FurnitureSurfaceUniform, KIND_CLOTH, KIND_COSMOS, KIND_LACQUER,
	KIND_LAVA, KIND_MARBLE, KIND_METAL, KIND_ORNATE, KIND_ROCKADDER, KIND_SCALES, KIND_SOFT,
	KIND_WOOD, RECIPE_FURNITURE_CLOTH, RECIPE_FURNITURE_COSMOS, RECIPE_FURNITURE_LACQUER,
	RECIPE_FURNITURE_LAVA, RECIPE_FURNITURE_MARBLE, RECIPE_FURNITURE_METAL,
	RECIPE_FURNITURE_ORNATE, RECIPE_FURNITURE_ROCKADDER, RECIPE_FURNITURE_SCALES,
	RECIPE_FURNITURE_SOFT, RECIPE_FURNITURE_WOOD,
};
pub use material_lib::{
	init_furniture_material_caches, FurnitureMaterialLib, FurnitureMaterialRefPlugin,
	FurnitureStandaloneMaterialLib, FurnitureSurfaceMaterialRefCache,
};

/// Registers furniture-surface materials used by furniture [`material_ref::MaterialLib`]s.
pub struct FurnitureShadersPlugin;

impl Plugin for FurnitureShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(FurnitureSurfaceMaterialPlugin);
	}
}
