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
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::{HighBushShoots, QuantizedPlant, TuftPatch};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, BushScrubCell, DRY_TUFT, DRY_TUFT_PATCH, GREEN_TUFT, GREEN_TUFT_PATCH,
	SAPLING_BUSH, SMALL_BUSH,
};
use crate::grove::vc_tuft::{material_from_palette, patch_variant_index, unit_plant_from_grown};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_blade_tuft_plant, remixed_bush_plant, remixed_tuft_plant,
	stick_material_from_palette, CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct BushScrubParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<BushScrubCell>,

	#[arg(
		long,
		default_value = "0,1.0,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Leaf Surface Noise",
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(long, default_value_t = 100)]
	pub patch_variants: u32,
}

impl Default for BushScrubParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(BushScrubParams, BushScrubCell);

impl BushScrubParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> BushScrub {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> BushScrub {
		BushScrub::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			self.leaf_surface_noise,
			self.patch_variants,
			self.tree_variants,
			&self.extent,
		)
	}
}

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.06, 1)
}

remixed_blade_tuft_plant!(DryTuft, DRY_TUFT, default_foliage());
remixed_blade_tuft_plant!(GreenTuft, GREEN_TUFT, default_foliage());
remixed_tuft_plant!(DryTuftPatch, DRY_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(GreenTuftPatch, GREEN_TUFT_PATCH, default_foliage());
remixed_bush_plant!(BushScrubSmall, SMALL_BUSH);
remixed_bush_plant!(BushScrubSapling, SAPLING_BUSH);

#[derive(Clone)]
enum BushScrubKind {
	Tuft(Arc<TuftPatch>),
	Bush(Arc<HighBushShoots>),
}

#[derive(Clone)]
struct BushScrubPlant {
	placement: Placement,
	kind: BushScrubKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct BushScrub {
	plants: Arc<[BushScrubPlant]>,
	structural_center: Vec3,
	footprint_radius: f32,
	pub extent: GroveExtent,
}

impl BushScrub {
	pub fn from_placements(
		placements: &[GroveCellVariant<BushScrubCell>],
		grove_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		patch_variants: u32,
		tree_variants: u32,
		extent: &GroveExtent,
	) -> Self {
		let patch_variants = patch_variants.max(1);
		let tree_variants = tree_variants.max(1);
		let plants: Arc<[BushScrubPlant]> = placements
			.iter()
			.map(|placed| {
				grow_plant(placed, grove_noise, leaf_surface_noise, patch_variants, tree_variants)
			})
			.collect::<Vec<_>>()
			.into();
		let (structural_center, footprint_radius) = grove_structural_footprint(extent);
		Self { plants, structural_center, footprint_radius, extent: *extent }
	}

	pub fn is_empty(&self) -> bool {
		self.plants.is_empty()
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
			Some(match &plant.kind {
				BushScrubKind::Tuft(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				BushScrubKind::Bush(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
			})
		})]
	}

	fn canopy_sites(&self) -> Vec<CanopyProxySite> {
		self.plants
			.iter()
			.filter_map(|plant| match &plant.kind {
				BushScrubKind::Bush(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
				}
				BushScrubKind::Tuft(t) => {
					Some(tuft_proxy_site(t, plant.placement, &plant.ball_material))
				}
			})
			.collect()
	}
}

fn tuft_proxy_site(
	patch: &TuftPatch,
	placement: Placement,
	material: &MaterialRef,
) -> CanopyProxySite {
	let scale = placement.scale.abs().max_element().max(1e-4);
	let height = (patch.shape.blade_length * scale).max(0.15);
	let footprint = (patch.patch_extent_xz * 0.5 * scale).max(height * 0.35);
	CanopyProxySite::from_radius(
		placement.translation + Vec3::Y * (height * 0.4),
		footprint.max(0.25),
		material.clone(),
	)
}

fn grow_plant(
	placed: &GroveCellVariant<BushScrubCell>,
	grove_noise: NoiseParams,
	leaf_surface_noise: NoiseParams,
	patch_variants: u32,
	tree_variants: u32,
) -> BushScrubPlant {
	match placed.variant {
		BushScrubCell::DryTuft
		| BushScrubCell::GreenTuft
		| BushScrubCell::DryTuftPatch
		| BushScrubCell::GreenTuftPatch => {
			let variant = patch_variant_index(placed.position, patch_variants);
			let (patch, world_size) = match placed.variant {
				BushScrubCell::DryTuft => DryTuft::grow_num(variant),
				BushScrubCell::GreenTuft => GreenTuft::grow_num(variant),
				BushScrubCell::DryTuftPatch => DryTuftPatch::grow_num(variant),
				BushScrubCell::GreenTuftPatch => GreenTuftPatch::grow_num(variant),
				_ => unreachable!("tuft cells only"),
			};
			let material = material_from_palette(
				placed.variant.palette_mix(),
				placed.position,
				leaf_surface_noise,
			);
			let (placement, patch, material) =
				unit_plant_from_grown(patch, world_size, placed.position, placed.scale, material);
			BushScrubPlant {
				placement,
				kind: BushScrubKind::Tuft(patch),
				stick_material: MaterialRef::default(),
				ball_material: material.clone(),
				frond_material: material,
			}
		}
		BushScrubCell::SmallBush | BushScrubCell::SaplingBush => {
			let variant = patch_variant_index(placed.position, tree_variants);
			let palette_noise = placement_noise(grove_noise, placed.position);
			let stick_seed = palette_noise.seed;
			let canopy_seed = palette_noise.seed.wrapping_add(31);
			let (bush, world_size) = match placed.variant {
				BushScrubCell::SmallBush => BushScrubSmall::grow_num(variant),
				BushScrubCell::SaplingBush => BushScrubSapling::grow_num(variant),
				_ => unreachable!("Bush item is only SmallBush or SaplingBush"),
			};
			BushScrubPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: BushScrubKind::Bush(bush),
				stick_material: stick_material_from_palette(
					Some(placed.variant.stick_palette_mix()),
					stick_seed,
				),
				ball_material: canopy_ball_material_from_palette(
					Some(placed.variant.canopy_palette_mix()),
					canopy_seed,
				),
				frond_material: frond_material_from_palette(
					Some(placed.variant.canopy_palette_mix()),
					canopy_seed,
				),
			}
		}
	}
}

crate::impl_woody_grove_lod!(BushScrub, WOODY_LOD);

#[cfg(test)]
mod tests;
