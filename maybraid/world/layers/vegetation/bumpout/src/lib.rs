//! Terrain-mesh bump outs for Chico ground cover and distant canopy mass.
//!
//! A [`BumpOut`] and ordinary terrain presenter may carry the same
//! [`terrain_chunk_ref::TerrainChunkRef`]. Lazy terrain fulfillment gives both entities the same
//! mesh handle, while [`BumpOutMaterial`] performs cell-profile blending, vertical displacement,
//! and fragment dropout in the shader.
//!
//! GPU uniforms are a shared raster/scalar pad. This shader reads density / bite / height
//! on channels 0–4 and style on scalars 0–6.

mod bump_out;
mod material;
mod neighborhood;

use bevy::prelude::{App, Plugin};
use vegetation_shaders::BumpOutMaterialPlugin;

pub use bump_out::BumpOut;
pub use material::{
	init_bump_out_material_caches, BumpOutMaterialLib, BumpOutMaterialRefCache,
	BumpOutMaterialRefPlugin, BumpOutStandaloneMaterialLib,
};
pub use neighborhood::{
	BumpOutNeighborhood, BumpOutStyle, BUMP_OUT_NEIGHBORHOOD_SAMPLES, BUMP_OUT_NEIGHBORHOOD_WIDTH,
};
pub use vegetation_shaders::{
	BumpOutMaterial, BumpOutUniform, BUMP_OUT_MATERIAL, RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE,
	RASTER_BITE_SIZE_DEVIATION, RASTER_DENSITY, RASTER_HEIGHT_DEVIATION,
};

/// Registers the bump-out shader and caches. Does not install a [`material_ref::MaterialRefPlugin`].
///
/// Standalone apps add [`BumpOutMaterialRefPlugin`]. Composed apps (vegetation / world)
/// install one parent lib instead.
pub struct BumpOutPlugin;

impl Plugin for BumpOutPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<BumpOutMaterialPlugin>() {
			app.add_plugins(BumpOutMaterialPlugin);
		}
		init_bump_out_material_caches(app);
	}
}
