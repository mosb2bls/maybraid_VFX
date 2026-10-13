//! Furniture recipes, then Chico foliage, then Standard.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use furniture_shaders::{
	init_furniture_material_caches, FurnitureMaterialLib, FurnitureShadersPlugin,
};
use material_ref::{
	material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};
use vegetation_shaders::{
	init_vegetation_material_caches, VegetationMaterialLib, VegetationShadersPlugin,
};

/// Isolated catalog lib: furniture surfaces, then leaf / stick / frond.
#[derive(SystemParam)]
pub struct PlaygroundMaterialLib<'w> {
	pub furniture: FurnitureMaterialLib<'w>,
	pub chico: VegetationMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for PlaygroundMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.furniture.try_fulfill(entity, material_ref, commands)
			|| self.chico.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Caches + shaders + one [`MaterialRefPlugin`] for the furniture catalog.
pub struct PlaygroundMaterialRefPlugin;

impl Plugin for PlaygroundMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((FurnitureShadersPlugin, VegetationShadersPlugin));
		init_furniture_material_caches(app);
		init_vegetation_material_caches(app);
		app.init_resource::<StandardMaterialRefCache>();
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<PlaygroundMaterialLib<'_>>::default());
	}
}
