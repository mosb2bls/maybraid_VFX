//! Bevy plugins: vegetation view stack plus forest generate / present / cull.

use bevy::prelude::*;
use scene_ref::SceneRefPlugin;
use vegetation_components::VegetationProceduralPlugin;
use vegetation_shaders::{
	init_vegetation_material_caches, VegetationMaterialRefPlugin, VegetationShadersPlugin,
};

use crate::view::VegetationLodRefreshPlugin;

/// Shaders, kit caches, and Avian LOD refresh for forest / grove hosts.
pub struct VegetationViewPlugin;

impl Plugin for VegetationViewPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<VegetationProceduralPlugin>() {
			app.add_plugins(VegetationProceduralPlugin);
		}
		if !app.is_plugin_added::<VegetationLodRefreshPlugin>() {
			app.add_plugins(VegetationLodRefreshPlugin);
		}
		if !app.is_plugin_added::<VegetationShadersPlugin>() {
			app.add_plugins(VegetationShadersPlugin);
		}
		if !app.is_plugin_added::<VegetationMaterialRefPlugin>() {
			app.add_plugins(VegetationMaterialRefPlugin);
		}
		init_vegetation_material_caches(app);
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
	}
}

/// Register shaders, kit caches, and vegetation LOD refresh if missing.
pub fn register_vegetation_view(app: &mut App) {
	if !app.is_plugin_added::<VegetationViewPlugin>() {
		app.add_plugins(VegetationViewPlugin);
	}
}
