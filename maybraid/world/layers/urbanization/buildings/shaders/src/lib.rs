//! Richmond building shaders: named [`MaterialRef`] urban-surface recipes.

use bevy::prelude::*;

mod column_surface;
mod material_lib;
mod urban_surface;

pub use column_surface::shaft::{
	PLACEHOLDER_AUTHORED_D, PLACEHOLDER_SHAFT_HEIGHT_M, PLACEHOLDER_SHAFT_RADIUS_M,
	PLACEHOLDER_SHAFT_TAPER, PLACEHOLDER_TAPERED_SHAFT_HEIGHT_IN_D, PlaceholderShaft,
	placeholder_shaft_mesh, placeholder_tapered_shaft_mesh,
};
pub use column_surface::{
	ColumnSurfaceMaterial, ColumnSurfaceMaterialPlugin, ColumnSurfaceUniform, DEFAULT_FLUTE_COUNT,
	DEFAULT_POM_FADE_END_M, DEFAULT_POM_FADE_START_M, DEFAULT_POM_MAX_STEPS,
	DEFAULT_RELIEF_DEPTH_D, KIND_DORIC, RECIPE_DORIC_COLUMN, RECIPE_DORIC_COLUMN_SHAFT_KIT,
	SCALAR_FLAT,
	doric_column_flat_ref, doric_column_kit_shaft_flat_ref, doric_column_kit_shaft_ref,
	doric_column_ref, is_column_surface_recipe,
};
pub use material_lib::{
	ColumnSurfaceMaterialLib, ColumnSurfaceMaterialRefCache, UrbanMaterialLib,
	UrbanMaterialRefPlugin, UrbanSurfaceMaterialLib, UrbanSurfaceMaterialRefCache,
	init_urban_material_caches,
};
pub use urban_surface::{
	KIND_HAY, KIND_IRON, KIND_STONE, KIND_STUCCO, KIND_TERRACOTTA, KIND_WOOD, RECIPE_HAY,
	RECIPE_IRON, RECIPE_STONE, RECIPE_STUCCO, RECIPE_TERRACOTTA, RECIPE_WOOD, UrbanSurfaceKind,
	UrbanSurfaceMaterial, UrbanSurfaceMaterialPlugin, UrbanSurfaceUniform, is_urban_surface_recipe,
};

/// Registers urban-surface materials used by Richmond [`material_ref::MaterialLib`]s.
pub struct BuildingShadersPlugin;

impl Plugin for BuildingShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(UrbanSurfaceMaterialPlugin);
		app.add_plugins(ColumnSurfaceMaterialPlugin);
	}
}
