use super::WOODY_LOD;
use bevy::prelude::*;
use std::sync::Arc;

#[cfg(test)]
use bevy::math::bounding::Aabb3d;
use clap::Args;
use lod::lod_ref::LodRef;
#[cfg(test)]
use lod::scene::LodScene;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::{BuildWithNoise, NoiseParams};
use sbs_trees::{NorthernConifer, NorthernConiferParams, QuantizedPlant};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, ChristmasTaigaCell, ChristmasTaigaNorthernConifer, CHRISTMAS_NORTHERN_CONIFER,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_column, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, stick_material_from_palette, unit_build_noise, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct ChristmasTaigaParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<ChristmasTaigaCell>,
}

impl Default for ChristmasTaigaParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.50, steepness: 0.30 }),
		}
	}
}

crate::impl_grove_preview_params!(ChristmasTaigaParams, ChristmasTaigaCell);

impl ChristmasTaigaParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> ChristmasTaiga {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> ChristmasTaiga {
		ChristmasTaiga::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn christmas_northern_unit(
	authored: &ChristmasTaigaNorthernConifer,
	num: u32,
) -> (NorthernConifer, f32) {
	let samples = authored.build_with_noise(unit_build_noise(num));
	let mut params = NorthernConiferParams::default();
	params.geometry = samples.geometry;
	params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
	params.splay_spawn_fraction = samples.splay_spawn_fraction;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct ChristmasNorthern;

impl QuantizedPlant for ChristmasNorthern {
	type Unit = NorthernConifer;

	fn build_unit(num: u32) -> (NorthernConifer, f32) {
		christmas_northern_unit(&CHRISTMAS_NORTHERN_CONIFER, num)
	}
}

#[derive(Clone)]
pub struct ChristmasTaigaPlant {
	pub placement: Placement,
	pub(crate) tree: Arc<NorthernConifer>,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct ChristmasTaiga {
	pub plants: Arc<[ChristmasTaigaPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl ChristmasTaiga {
	pub fn from_placements(
		placements: &[GroveCellVariant<ChristmasTaigaCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[ChristmasTaigaPlant]> = placements
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
				Arc::clone(&plant.tree),
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
				canopy_proxy_column(&plant.tree, plant.placement, &plant.ball_material)
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<ChristmasTaigaCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> ChristmasTaigaPlant {
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

	let (tree, world_size) = ChristmasNorthern::grow_num(variant);

	ChristmasTaigaPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		tree,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(ChristmasTaiga, WOODY_LOD);

#[cfg(test)]
mod tests;
