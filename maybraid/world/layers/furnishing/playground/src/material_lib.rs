//! Furniture recipes, then Chico foliage, then Standard.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use furniture_shaders::{
	init_furniture_material_caches, FurnitureMaterialLib, FurnitureShadersPlugin,
};
use material_ref::{
	MaterialLib, MaterialRef, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
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

/// Furniture playground composition-root plugin: shaders, caches, and
/// [`PlaygroundMaterialLib`] as the unique resolver.
pub struct PlaygroundMaterialRefPlugin;

impl Plugin for PlaygroundMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((FurnitureShadersPlugin, VegetationShadersPlugin));
		init_furniture_material_caches(app);
		init_vegetation_material_caches(app);
		app.init_resource::<StandardMaterialRefCache>();
		app.add_plugins(MaterialRefPlugin::<PlaygroundMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use furniture_shaders::{
		FurnitureSurfaceMaterial, FurnitureSurfaceMaterialRefCache, RECIPE_FURNITURE_WOOD,
	};
	use material_ref::{
		MaterialRef, MaterialRefApplied, MaterialRefPlugin, MaterialRefRoot,
		StandardMaterialRefCache,
	};
	use vegetation_shaders::{
		FrondMaterial, FrondMaterialRefCache, LeafMaterial, LeafMaterialRefCache, StickMaterial,
		StickMaterialRefCache,
	};

	use super::*;

	#[test]
	fn playground_lib_fulfills_furniture_then_leaf_then_standard() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<FurnitureSurfaceMaterial>()
			.init_asset::<LeafMaterial>()
			.init_asset::<StickMaterial>()
			.init_asset::<FrondMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.init_resource::<FurnitureSurfaceMaterialRefCache>()
			.init_resource::<LeafMaterialRefCache>()
			.init_resource::<StickMaterialRefCache>()
			.init_resource::<FrondMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<PlaygroundMaterialLib<'_>>::default());

		let furniture = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named(RECIPE_FURNITURE_WOOD)))
			.id();
		let leaf = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named("CHICO_LEAF_MATERIAL")))
			.id();
		let fallback = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named("tuft"))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(furniture).is_some());
		assert!(app.world().get::<MeshMaterial3d<FurnitureSurfaceMaterial>>(furniture).is_some());
		assert!(app.world().get::<MeshMaterial3d<LeafMaterial>>(leaf).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(fallback).is_some());
	}
}
