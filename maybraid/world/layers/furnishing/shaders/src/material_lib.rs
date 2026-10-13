//! [`MaterialLib`] for furniture-surface shaders.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

use crate::{is_furniture_surface_recipe, FurnitureSurfaceMaterial};

/// Cache of resolved [`FurnitureSurfaceMaterial`] handles.
pub type FurnitureSurfaceMaterialRefCache = MaterialRefCache<FurnitureSurfaceMaterial>;

/// Inserts furniture-surface material caches. Idempotent.
pub fn init_furniture_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<FurnitureSurfaceMaterialRefCache>();
}

/// Claims furniture recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct FurnitureMaterialLib<'w> {
	pub furniture_materials: ResMut<'w, Assets<FurnitureSurfaceMaterial>>,
	pub furniture_cache: ResMut<'w, FurnitureSurfaceMaterialRefCache>,
}

impl FurnitureMaterialLib<'_> {
	fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<FurnitureSurfaceMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.furniture_cache.get(&key) {
			return handle;
		}
		let handle = self
			.furniture_materials
			.add(FurnitureSurfaceMaterial::from_material_ref(material_ref));
		self.furniture_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for FurnitureMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if is_furniture_surface_recipe(name) => {
				let handle = self.resolve(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Furniture recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct FurnitureStandaloneMaterialLib<'w> {
	pub furniture: FurnitureMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for FurnitureStandaloneMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.furniture.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers caches + [`MaterialRefPlugin`] for [`FurnitureStandaloneMaterialLib`].
pub struct FurnitureMaterialRefPlugin;

impl Plugin for FurnitureMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_furniture_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<FurnitureStandaloneMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use material_ref::{MaterialRef, MaterialRefApplied, MaterialRefPlugin, MaterialRefRoot};

	use super::*;
	use crate::{FurnitureSurfaceMaterial, RECIPE_FURNITURE_WOOD};

	#[test]
	fn furniture_lib_fulfills_wood_as_furniture_surface() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<FurnitureSurfaceMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.init_resource::<FurnitureSurfaceMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<FurnitureMaterialLib<'_>>::default());

		let entity = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named(RECIPE_FURNITURE_WOOD)))
			.id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<FurnitureSurfaceMaterial>>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_none());
	}
}
