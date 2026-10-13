use super::WOODY_LOD;
use std::sync::Arc;

#[cfg(test)]
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use clap::Args;
use lod::lod_ref::LodRef;
#[cfg(test)]
use lod::scene::LodScene;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;
use sbs_trees::{HighBushShoots, QuantizedPlant};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, SpottyBushesCell, DENSE_SPOT_BUSH, DRY_SPOT_BUSH, FLOWERING_SPOT_BUSH,
	GREEN_SPOT_BUSH,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_bush_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct SpottyBushesParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<SpottyBushesCell>,
}

impl Default for SpottyBushesParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(SpottyBushesParams, SpottyBushesCell);

impl SpottyBushesParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> SpottyBushes {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> SpottyBushes {
		SpottyBushes::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_bush_plant!(SpottyGreen, GREEN_SPOT_BUSH);
remixed_bush_plant!(SpottyDry, DRY_SPOT_BUSH);
remixed_bush_plant!(SpottyDense, DENSE_SPOT_BUSH);
remixed_bush_plant!(SpottyFlowering, FLOWERING_SPOT_BUSH);

#[derive(Clone)]
pub struct SpottyBushesPlant {
	pub placement: Placement,
	pub(crate) bush: Arc<HighBushShoots>,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct SpottyBushes {
	pub plants: Arc<[SpottyBushesPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl SpottyBushes {
	pub fn from_placements(
		placements: &[GroveCellVariant<SpottyBushesCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[SpottyBushesPlant]> = placements
			.iter()
			.map(|placed| grow_plant(placed, grove_noise, tree_variants))
			.collect::<Vec<_>>()
			.into();
		let (structural_center, footprint_radius) = grove_structural_footprint(extent);
		Self { plants, structural_center, footprint_radius, extent: *extent }
	}

	fn nest_plant_chunks(&self, lod_ref: &LodRef, level: LodSceneLevel) -> Vec<SceneChunk> {
		if self.plants.is_empty() {
			return Vec::new();
		}
		let n = self.plants.len();
		let plants = Arc::clone(&self.plants);
		let prev = *lod_ref.previous_transform;
		let curr = *lod_ref.current_transform;
		let bounds = *lod_ref.bounds;
		let entity = lod_ref.entity;
		let mut index = 0usize;
		vec![lazy_flattened_plant_chunks(n, move || {
			if index >= plants.len() {
				return None;
			}
			let plant = &plants[index];
			index += 1;
			let plant_lod = LodRef {
				entity,
				previous_transform: &prev,
				current_transform: &curr,
				bounds: &bounds,
			};
			Some(nest_flattened_plant_chunk(
				Arc::clone(&plant.bush),
				plant.placement,
				&plant.stick_material,
				&plant.ball_material,
				&plant.frond_material,
				&plant_lod,
				level,
			))
		})]
	}

	fn canopy_sites(&self) -> Vec<CanopyProxySite> {
		self.plants
			.iter()
			.filter_map(|plant| {
				canopy_proxy_site(&plant.bush, plant.placement, &plant.ball_material)
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<SpottyBushesCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> SpottyBushesPlant {
	let variant = patch_variant_index(placed.position, tree_variants);
	let palette_noise = placement_noise(grove_noise, placed.position);
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	let stick_material =
		stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
	let ball_material =
		canopy_ball_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);
	let frond_material =
		frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

	let (bush, world_size) = match placed.variant {
		SpottyBushesCell::GreenSpotBush => SpottyGreen::grow_num(variant),
		SpottyBushesCell::DrySpotBush => SpottyDry::grow_num(variant),
		SpottyBushesCell::DenseSpotBush => SpottyDense::grow_num(variant),
		SpottyBushesCell::FloweringSpotBush => SpottyFlowering::grow_num(variant),
	};
	let placement = Placement::new(placed.position, 0.0)
		.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));

	SpottyBushesPlant { placement, bush, stick_material, ball_material, frond_material }
}

crate::impl_woody_grove_lod!(SpottyBushes, WOODY_LOD);

#[cfg(test)]
mod tests;
