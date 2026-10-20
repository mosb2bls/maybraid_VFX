//! character [`MaterialLib`]: face, then clothing shader recipes.
//!
//! [`CharacterMaterialLib`] is the composable domain lib. Standalone apps add
//! [`StandaloneCharacterMaterialRefPlugin`] so [`StandaloneCharacterMaterialLib`]
//! falls through to [`StandardMaterial`]. Maybraid World nests
//! [`CharacterMaterialLib`] and falls through to vegetation.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use character_items::{ClothingMaterial, FirearmMaterial};
use character_shaders::{ClothingShaderMaterial, FaceShaderKind, FaceShaderMaterial};
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Cache of resolved [`FaceShaderMaterial`] handles.
pub type FaceShaderMaterialRefCache = MaterialRefCache<FaceShaderMaterial>;

/// Inserts clothing and face material caches. Idempotent.
pub fn init_character_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ClothingShaderMaterialRefCache>()
		.init_resource::<FaceShaderMaterialRefCache>();
}

/// Claims face recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct FaceMaterialLib<'w> {
	pub face_materials: ResMut<'w, Assets<FaceShaderMaterial>>,
	pub face_cache: ResMut<'w, FaceShaderMaterialRefCache>,
}

impl FaceMaterialLib<'_> {
	fn resolve_face(&mut self, material_ref: &MaterialRef) -> Handle<FaceShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.face_cache.get(&key) {
			return handle;
		}
		let handle = self.face_materials.add(FaceShaderMaterial::from_material_ref(material_ref));
		self.face_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for FaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if FaceShaderKind::is_face_recipe(name) => {
				let handle = self.resolve_face(material_ref);
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Claims clothing recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ClothingMaterialLib<'w> {
	pub clothing_materials: ResMut<'w, Assets<ClothingShaderMaterial>>,
	pub clothing_cache: ResMut<'w, ClothingShaderMaterialRefCache>,
}

impl ClothingMaterialLib<'_> {
	fn resolve_clothing(&mut self, material_ref: &MaterialRef) -> Handle<ClothingShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.clothing_cache.get(&key) {
			return handle;
		}
		let handle = self
			.clothing_materials
			.add(ClothingShaderMaterial::from_material_ref(material_ref));
		self.clothing_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for ClothingMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name)
				if ClothingMaterial::is_clothing_recipe(name)
					|| FirearmMaterial::is_firearm_recipe(name) =>
			{
				let handle = self.resolve_clothing(material_ref);
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Face + clothing / firearm recipes. Does not fall through to [`StandardMaterial`].
///
/// Standalone apps use [`StandaloneCharacterMaterialLib`]. Composed apps (Maybraid
/// World) nest this and supply their own fallback.
#[derive(SystemParam)]
pub struct CharacterMaterialLib<'w> {
	pub face: FaceMaterialLib<'w>,
	pub clothing: ClothingMaterialLib<'w>,
}

impl MaterialLib for CharacterMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.face.try_fulfill(entity, material_ref, commands)
			|| self.clothing.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Menu / playground fulfill: character, then green [`StandardMaterial`].
#[derive(SystemParam)]
pub struct StandaloneCharacterMaterialLib<'w> {
	pub character: CharacterMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for StandaloneCharacterMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.character.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Idempotent clothing / face cache initializer. Does not install a resolver.
///
/// Composition roots install one [`MaterialRefPlugin`]: playgrounds use
/// [`StandaloneCharacterMaterialRefPlugin`]; Maybraid World nests
/// [`CharacterMaterialLib`] in its composed world lib.
pub struct CharacterMaterialRefPlugin;

impl Plugin for CharacterMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_character_material_caches(app);
	}
}

/// Playground / menu composition-root plugin: caches + [`StandaloneCharacterMaterialLib`].
///
/// Maybraid World must not add this; it installs its composed world lib instead.
pub struct StandaloneCharacterMaterialRefPlugin;

impl Plugin for StandaloneCharacterMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_character_material_caches(app);
		app.add_plugins(MaterialRefPlugin::<StandaloneCharacterMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use character_shaders::{FaceShaderMaterial, RECIPE_FACE_EYE};
	use material_ref::{
		material_ref_plugin_installed, MaterialRef, MaterialRefApplied, MaterialRefRoot,
	};

	use super::*;

	#[test]
	fn character_material_ref_plugin_is_cache_only() {
		let mut app = App::new();
		app.add_plugins(CharacterMaterialRefPlugin);
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<FaceShaderMaterialRefCache>());
		assert!(!material_ref_plugin_installed(&app));
	}

	#[test]
	fn standalone_character_lib_fulfills_face_then_standard() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<FaceShaderMaterial>()
			.init_asset::<ClothingShaderMaterial>()
			.add_plugins(StandaloneCharacterMaterialRefPlugin);

		let face = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named(RECIPE_FACE_EYE))).id();
		let other = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named("tuft"))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(face).is_some());
		assert!(app.world().get::<MeshMaterial3d<FaceShaderMaterial>>(face).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(face).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(other).is_some());
	}
}
