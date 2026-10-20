//! Composed [`MaterialLib`] for vegetation-on-terrain: bump-out + Chico + Standard.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{MaterialLib, MaterialRef, StandardMaterialLib, StandardMaterialRefCache};
use vegetation_bumpout::{init_bump_out_material_caches, BumpOutMaterialLib};
use vegetation_shaders::{init_vegetation_material_caches, VegetationMaterialLib};

/// Vegetation playground / world-view lib: bump-out, then leaf/stick/frond, then Standard.
#[derive(SystemParam)]
pub struct VegetationOnTerrainMaterialLib<'w> {
	pub bump_out: BumpOutMaterialLib<'w>,
	pub chico: VegetationMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for VegetationOnTerrainMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.bump_out.try_fulfill(entity, material_ref, commands)
			|| self.chico.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Idempotent vegetation-on-terrain cache initializer. Does not install a resolver.
///
/// Isolated vegetation hosts install [`MaterialRefPlugin`]`<`[`VegetationOnTerrainMaterialLib`]`>`
/// at their composition root. Maybraid World nests this lib in `WorldMaterialLib`.
pub struct VegetationOnTerrainMaterialRefPlugin;

impl Plugin for VegetationOnTerrainMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_vegetation_material_caches(app);
		init_bump_out_material_caches(app);
		app.init_resource::<StandardMaterialRefCache>();
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use material_ref::{
		material_ref_plugin_installed, MaterialRef, MaterialRefApplied, MaterialRefPlugin,
		MaterialRefRoot,
	};
	use vegetation_bumpout::BumpOutMaterial;
	use vegetation_shaders::LeafMaterial;

	use super::*;

	#[test]
	fn vegetation_on_terrain_plugin_is_cache_only() {
		let mut app = App::new();
		app.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		assert!(!material_ref_plugin_installed(&app));
	}

	#[test]
	fn vegetation_on_terrain_lib_fulfills_bump_out_leaf_then_standard() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<BumpOutMaterial>()
			.init_asset::<LeafMaterial>()
			.init_asset::<vegetation_shaders::StickMaterial>()
			.init_asset::<vegetation_shaders::FrondMaterial>()
			.add_plugins(VegetationOnTerrainMaterialRefPlugin)
			.add_plugins(MaterialRefPlugin::<VegetationOnTerrainMaterialLib<'_>>::default());

		let bump = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named(vegetation_bumpout::BUMP_OUT_MATERIAL)))
			.id();
		let leaf = app
			.world_mut()
			.spawn(MaterialRefRoot(MaterialRef::named("CHICO_LEAF_MATERIAL")))
			.id();
		let fallback = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named("tuft"))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(bump).is_some());
		assert!(app.world().get::<MeshMaterial3d<BumpOutMaterial>>(bump).is_some());
		assert!(app.world().get::<MeshMaterial3d<LeafMaterial>>(leaf).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(fallback).is_some());
	}
}
