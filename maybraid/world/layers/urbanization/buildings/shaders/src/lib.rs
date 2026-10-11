//! Richmond building shaders: named [`MaterialRef`] urban-surface recipes.

use bevy::prelude::*;

mod column_surface;
mod material_lib;
mod urban_surface;

pub use column_surface::shaft::{
	placeholder_shaft_mesh, PlaceholderShaft, PLACEHOLDER_SHAFT_HEIGHT_M,
	PLACEHOLDER_SHAFT_RADIUS_M,
};
pub use column_surface::{
	doric_column_ref, is_column_surface_recipe, ColumnSurfaceMaterial, ColumnSurfaceMaterialPlugin,
	ColumnSurfaceUniform, DEFAULT_FLUTE_COUNT, DEFAULT_POM_FADE_END_M, DEFAULT_POM_FADE_START_M,
	DEFAULT_POM_MAX_STEPS, DEFAULT_RELIEF_DEPTH_M, RECIPE_DORIC_COLUMN,
};
pub use material_lib::{
	init_urban_material_caches, ColumnSurfaceMaterialLib, ColumnSurfaceMaterialRefCache,
	UrbanMaterialLib, UrbanMaterialRefPlugin, UrbanSurfaceMaterialLib,
	UrbanSurfaceMaterialRefCache,
};
pub use urban_surface::{
	is_urban_surface_recipe, UrbanSurfaceKind, UrbanSurfaceMaterial, UrbanSurfaceMaterialPlugin,
	UrbanSurfaceUniform, KIND_HAY, KIND_IRON, KIND_STONE, KIND_STUCCO, KIND_TERRACOTTA, KIND_WOOD,
	RECIPE_HAY, RECIPE_IRON, RECIPE_STONE, RECIPE_STUCCO, RECIPE_TERRACOTTA, RECIPE_WOOD,
};

/// Registers urban-surface materials used by Richmond [`material_ref::MaterialLib`]s.
pub struct BuildingShadersPlugin;

impl Plugin for BuildingShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(UrbanSurfaceMaterialPlugin);
		app.add_plugins(ColumnSurfaceMaterialPlugin);
	}
}
