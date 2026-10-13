//! [`MaterialLib`] for bump-out recipes. The Bevy material lives in `chico-vegetation-shaders`.

use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};
use vegetation_shaders::{BumpOutMaterial, BUMP_OUT_MATERIAL};

pub type BumpOutMaterialRefCache = MaterialRefCache<BumpOutMaterial>;

/// Inserts bump-out material caches. Idempotent.
pub fn init_bump_out_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<BumpOutMaterialRefCache>();
}

/// Claims `"chico_bump_out"` only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct BumpOutMaterialLib<'w> {
	pub materials: ResMut<'w, Assets<BumpOutMaterial>>,
	pub cache: ResMut<'w, BumpOutMaterialRefCache>,
}

impl BumpOutMaterialLib<'_> {
	pub fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<BumpOutMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.cache.get(&key) {
			return handle;
		}
		let handle = self.materials.add(BumpOutMaterial::from_material_ref(material_ref));
		self.cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for BumpOutMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if name == BUMP_OUT_MATERIAL => {
				let handle = self.resolve(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert((MeshMaterial3d(handle), NotShadowCaster));
				true
			}
			_ => false,
		}
	}
}

/// Bump-out crate standalone lib: bump-out recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct BumpOutStandaloneMaterialLib<'w> {
	pub bump_out: BumpOutMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for BumpOutStandaloneMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.bump_out.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers caches and deferred [`MaterialRef`] fulfillment for a standalone bump-out app.
///
/// Vegetation / world apps that compose several domain libs should call
/// [`init_bump_out_material_caches`] and skip this plugin.
pub struct BumpOutMaterialRefPlugin;

impl Plugin for BumpOutMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_bump_out_material_caches(app);
		if material_ref::material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<BumpOutStandaloneMaterialLib<'_>>::default());
	}
}

#[cfg(test)]
mod tests {
	use procedural_common::NoiseParams;
	use vegetation_shaders::{
		BumpOutUniform, RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION,
		RASTER_DENSITY, RASTER_HEIGHT_DEVIATION,
	};

	use super::*;
	use crate::{BumpOutNeighborhood, BumpOutStyle};

	#[test]
	fn material_ref_maps_raster_channels() {
		let neighborhood = BumpOutNeighborhood::new(
			[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
			[10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
			[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
			[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
			[0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.3],
		);
		let material_ref = neighborhood.material_ref(
			[Color::srgb(0.1, 0.5, 0.2)],
			NoiseParams::from_scalar(17.0, 0.2, 1.5, 2),
		);
		let material_ref = BumpOutStyle::new(0.07, 0.8, 0.3)
			.with_cheese(0.65, 1.4)
			.with_fragment_height(5.0, 0.75)
			.apply_to(material_ref);
		let uniform = BumpOutUniform::from_material_ref(&material_ref);

		assert_eq!(uniform.rasters[RASTER_DENSITY][1], Vec4::new(0.3, 0.4, 0.5, 0.0));
		assert_eq!(uniform.rasters[RASTER_BITE_SIZE][0], Vec4::new(10.0, 11.0, 12.0, 0.0));
		assert_eq!(uniform.rasters[RASTER_BITE_SIZE_DEVIATION][2], Vec4::new(0.6, 0.7, 0.8, 0.0));
		assert_eq!(uniform.rasters[RASTER_AVERAGE_HEIGHT][2], Vec4::new(7.0, 8.0, 9.0, 0.0));
		assert_eq!(uniform.rasters[RASTER_HEIGHT_DEVIATION][1], Vec4::new(0.8, 0.9, 1.0, 0.0));
		assert!((uniform.noise.y - 1.5).abs() < 1e-6);
		assert_eq!(uniform.scalars[0], Vec4::new(0.07, 0.8, 0.3, 0.65));
		assert_eq!(uniform.scalars[1], Vec4::new(1.4, 5.0, 0.75, 0.0));
	}
}
