//! character [`MaterialLib`]: face, then clothing shader recipes.
//!
//! [`CharacterMaterialLib`] is the composable domain lib. Standalone apps
//! ([`CharacterMaterialRefPlugin`]) add [`StandardMaterial`] via
//! [`StandaloneCharacterMaterialLib`]. Maybraid World nests [`CharacterMaterialLib`]
//! and falls through to vegetation.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use character_items::{ClothingMaterial, FirearmMaterial};
use character_shaders::{ClothingShaderMaterial, FaceShaderKind, FaceShaderMaterial};
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
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

/// Registers clothing / face caches + [`MaterialRefPlugin`] for
/// [`StandaloneCharacterMaterialLib`].
///
/// Composed apps that already installed a fulfill plugin (Maybraid World) skip
/// here; they nest [`CharacterMaterialLib`] instead.
pub struct CharacterMaterialRefPlugin;

impl Plugin for CharacterMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_character_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<StandaloneCharacterMaterialLib<'_>>::default());
	}
}
