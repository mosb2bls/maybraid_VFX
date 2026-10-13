//! Composed [`MaterialLib`] for vegetation-on-terrain: bump-out + Chico + Standard.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{
	material_ref_plugin_installed, MaterialLib, MaterialRef, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};
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

/// Caches + one [`MaterialRefPlugin`] for [`VegetationOnTerrainMaterialLib`].
///
/// Skips fulfill when a parent app (Maybraid World) already installed a composed lib.
pub struct VegetationOnTerrainMaterialRefPlugin;

impl Plugin for VegetationOnTerrainMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_vegetation_material_caches(app);
		init_bump_out_material_caches(app);
		app.init_resource::<StandardMaterialRefCache>();
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<VegetationOnTerrainMaterialLib<'_>>::default());
	}
}
