//! [`MaterialLib`] for Richmond urban-surface shaders.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

use crate::{
	is_column_surface_recipe, is_urban_surface_recipe, ColumnSurfaceMaterial, UrbanSurfaceMaterial,
};

/// Cache of resolved [`UrbanSurfaceMaterial`] handles.
pub type UrbanSurfaceMaterialRefCache = MaterialRefCache<UrbanSurfaceMaterial>;
/// Cache of resolved [`ColumnSurfaceMaterial`] handles.
pub type ColumnSurfaceMaterialRefCache = MaterialRefCache<ColumnSurfaceMaterial>;

/// Inserts urban-surface material caches. Idempotent.
pub fn init_urban_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<UrbanSurfaceMaterialRefCache>()
		.init_resource::<ColumnSurfaceMaterialRefCache>();
}

/// Claims urban recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct UrbanSurfaceMaterialLib<'w> {
	pub urban_materials: ResMut<'w, Assets<UrbanSurfaceMaterial>>,
	pub urban_cache: ResMut<'w, UrbanSurfaceMaterialRefCache>,
	pub column: ColumnSurfaceMaterialLib<'w>,
}

/// Claims column-surface recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ColumnSurfaceMaterialLib<'w> {
	pub column_materials: ResMut<'w, Assets<ColumnSurfaceMaterial>>,
	pub column_cache: ResMut<'w, ColumnSurfaceMaterialRefCache>,
}

impl ColumnSurfaceMaterialLib<'_> {
	fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<ColumnSurfaceMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.column_cache.get(&key) {
			return handle;
		}
		let handle = self
			.column_materials
			.add(ColumnSurfaceMaterial::from_material_ref(material_ref));
		self.column_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for ColumnSurfaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if is_column_surface_recipe(name) => {
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

impl UrbanSurfaceMaterialLib<'_> {
	fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<UrbanSurfaceMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.urban_cache.get(&key) {
			return handle;
		}
		let handle =
			self.urban_materials.add(UrbanSurfaceMaterial::from_material_ref(material_ref));
		self.urban_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for UrbanSurfaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if is_urban_surface_recipe(name) => {
				let handle = self.resolve(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => self.column.try_fulfill(entity, material_ref, commands),
		}
	}
}

/// Urban recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct UrbanMaterialLib<'w> {
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for UrbanMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.urban.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers caches + [`MaterialRefPlugin`] for [`UrbanMaterialLib`].
pub struct UrbanMaterialRefPlugin;

impl Plugin for UrbanMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_urban_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<UrbanMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use material_ref::{MaterialRef, MaterialRefApplied, MaterialRefPlugin, MaterialRefRoot};

	use super::*;
	use crate::{ColumnSurfaceMaterial, UrbanSurfaceMaterial, RECIPE_DORIC_COLUMN, RECIPE_WOOD};

	#[test]
	fn urban_lib_fulfills_wood_as_urban_surface() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<UrbanSurfaceMaterial>()
			.init_asset::<ColumnSurfaceMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.init_resource::<UrbanSurfaceMaterialRefCache>()
			.init_resource::<ColumnSurfaceMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<UrbanSurfaceMaterialLib<'_>>::default());

		let entity = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named(RECIPE_WOOD))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<UrbanSurfaceMaterial>>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_none());
	}

	#[test]
	fn urban_lib_fulfills_doric_column_as_column_surface() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<UrbanSurfaceMaterial>()
			.init_asset::<ColumnSurfaceMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.init_resource::<UrbanSurfaceMaterialRefCache>()
			.init_resource::<ColumnSurfaceMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<UrbanSurfaceMaterialLib<'_>>::default());

		let entity = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named(RECIPE_DORIC_COLUMN)))
			.id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<ColumnSurfaceMaterial>>(entity).is_some());
		assert!(app.world().get::<MeshMaterial3d<UrbanSurfaceMaterial>>(entity).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(entity).is_none());
	}
}
