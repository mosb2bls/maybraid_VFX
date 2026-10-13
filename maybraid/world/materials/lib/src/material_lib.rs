//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use building_shaders::{
	init_urban_material_caches, BuildingShadersPlugin, UrbanSurfaceMaterialLib,
};
use characters::material_lib::{init_character_material_caches, CharacterMaterialLib};
use characters::CharacterShadersPlugin;
use chico::VegetationOnTerrainMaterialLib;
use firearms::{
	init_energy_material_caches, init_muzzle_flame_caches, EnergyMaterialLib, EnergyMaterialPlugin,
	MuzzleFlameMaterialLib, MuzzleFlameMaterialPlugin,
};
use furniture_shaders::{
	init_furniture_material_caches, FurnitureMaterialLib, FurnitureShadersPlugin,
};
use material_ref::{material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin};
use vegetation_bumpout::init_bump_out_material_caches;
use vegetation_shaders::{init_vegetation_material_caches, VegetationShadersPlugin};

/// World-model lib: furniture kits, character face / clothing, Richmond urban
/// surfaces, the muzzle flame, energy looks, then vegetation and Standard.
///
/// Furniture recipes (`furniture_wood`, …) must be claimed **before** urban
/// `wood`. Urban recipes (`stucco`, `wood`, …), `muzzle_flame`, and the
/// `laser_*` energy recipes must be claimed **before** vegetation's
/// [`StandardMaterial`] fallback.
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub furniture: FurnitureMaterialLib<'w>,
	pub character: CharacterMaterialLib<'w>,
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub muzzle: MuzzleFlameMaterialLib<'w>,
	pub energy: EnergyMaterialLib<'w>,
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.furniture.try_fulfill(entity, material_ref, commands)
			|| self.character.try_fulfill(entity, material_ref, commands)
			|| self.urban.try_fulfill(entity, material_ref, commands)
			|| self.muzzle.try_fulfill(entity, material_ref, commands)
			|| self.energy.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Installs [`WorldMaterialLib`] as the single [`MaterialRefPlugin`] for Maybraid World.
///
/// Add this before vegetation layer `Present` so the nested vegetation
/// material plugin skips installing a second [`MaterialRefPlugin`].
/// [`characters::material_lib::CharacterMaterialRefPlugin`] and
/// [`building_shaders::UrbanMaterialRefPlugin`] also skip; face / urban
/// recipes are claimed here.
///
/// Does not register domain [`MaterialPlugin`]s — layers still do that.
/// Isolated hosts (the materials playground) also add
/// [`WorldMaterialsShadersPlugin`].
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_furniture_material_caches(app);
		init_character_material_caches(app);
		init_urban_material_caches(app);
		init_muzzle_flame_caches(app);
		init_energy_material_caches(app);
		init_vegetation_material_caches(app);
		init_bump_out_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}

/// Domain shader plugins used by [`WorldMaterialLib`] recipes.
///
/// Idempotent: layer stacks that already registered a shader plugin skip.
pub struct WorldMaterialsShadersPlugin;

impl Plugin for WorldMaterialsShadersPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<CharacterShadersPlugin>() {
			app.add_plugins(CharacterShadersPlugin);
		}
		if !app.is_plugin_added::<BuildingShadersPlugin>() {
			app.add_plugins(BuildingShadersPlugin);
		}
		if !app.is_plugin_added::<MuzzleFlameMaterialPlugin>() {
			app.add_plugins(MuzzleFlameMaterialPlugin);
		}
		if !app.is_plugin_added::<EnergyMaterialPlugin>() {
			app.add_plugins(EnergyMaterialPlugin);
		}
		if !app.is_plugin_added::<VegetationShadersPlugin>() {
			app.add_plugins(VegetationShadersPlugin);
		}
	}
}

/// Caches, fulfill, and domain shaders — enough for a standalone materials host.
pub struct WorldMaterialsPlugin;

impl Plugin for WorldMaterialsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((WorldMaterialsShadersPlugin, WorldMaterialRefPlugin));
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use characters::material_lib::{ClothingShaderMaterialRefCache, FaceShaderMaterialRefCache};

	use building_shaders::UrbanSurfaceMaterialRefCache;
	use firearms::{EnergyMaterialRefCache, MuzzleFlameMaterialRefCache};
	use furniture_shaders::FurnitureSurfaceMaterialRefCache;
	use vegetation_bumpout::BumpOutMaterialRefCache;
	use vegetation_shaders::{FrondMaterialRefCache, LeafMaterialRefCache, StickMaterialRefCache};

	use crate::material_lib::WorldMaterialRefPlugin;

	#[test]
	fn world_material_plugin_initializes_character_material_caches() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin);
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<FaceShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<UrbanSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<FurnitureSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<MuzzleFlameMaterialRefCache>());
		assert!(app.world().contains_resource::<EnergyMaterialRefCache>());
		assert!(app.world().contains_resource::<LeafMaterialRefCache>());
		assert!(app.world().contains_resource::<StickMaterialRefCache>());
		assert!(app.world().contains_resource::<FrondMaterialRefCache>());
		assert!(app.world().contains_resource::<BumpOutMaterialRefCache>());
	}
}
