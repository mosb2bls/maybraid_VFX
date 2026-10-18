use super::WOODY_LOD;
use std::sync::Arc;

use bevy::prelude::*;
use clap::Args;
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::{BuildWithNoise, NoiseParams};
use sbs_trees::{QuantizedPlant, StorybookTree, StorybookTreeParams};
use vegetation_components::{Placement, VegetationComponents};

use super::{definition, OrchardCell, OrchardStorybook, FRUITING_STORYBOOK, PALE_BLOOM_STORYBOOK};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, stick_material_from_palette, CanopyProxySite, FlatTerrainSample,
	GroveCellVariant, GroveExtent, GroveFrontend, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct OrchardParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<OrchardCell>,
}

impl Default for OrchardParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.10 }),
		}
	}
}

crate::impl_grove_preview_params!(OrchardParams, OrchardCell);

impl OrchardParams {
	pub fn build(&self) -> Orchard {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Orchard {
		Orchard::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn orchard_storybook_unit(authored: &OrchardStorybook, num: u32) -> (StorybookTree, f32) {
	let noise = variant_noise(GroveFrontend::default().noise, num);
	let mut params = StorybookTreeParams::default();
	params.geometry = authored.build_with_noise(noise);
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

/// Cache identity for fruiting orchard remixes of [`StorybookTree`].
struct OrchardFruiting;

/// Cache identity for pale-bloom orchard remixes of [`StorybookTree`].
struct OrchardPaleBloom;

impl QuantizedPlant for OrchardFruiting {
	type Unit = StorybookTree;

	fn build_unit(num: u32) -> (StorybookTree, f32) {
		orchard_storybook_unit(&FRUITING_STORYBOOK, num)
	}
}

impl QuantizedPlant for OrchardPaleBloom {
	type Unit = StorybookTree;

	fn build_unit(num: u32) -> (StorybookTree, f32) {
		orchard_storybook_unit(&PALE_BLOOM_STORYBOOK, num)
	}
}

#[derive(Clone)]
pub struct OrchardPlant {
	pub placement: Placement,
	pub(crate) tree: Arc<StorybookTree>,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct Orchard {
	pub plants: Arc<[OrchardPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl Orchard {
	pub fn from_placements(
		placements: &[GroveCellVariant<OrchardCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[OrchardPlant]> = placements
			.iter()
			.map(|placed| grow_plant(placed, grove_noise, tree_variants))
			.collect::<Vec<_>>()
			.into();
		let (structural_center, footprint_radius) = grove_structural_footprint(extent);
		Self { plants, structural_center, footprint_radius, extent: *extent }
	}

	/// High/Medium posed plants — one lazy producer so begin does not clone every tree.
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
				canopy_proxy_site(&plant.tree, plant.placement, &plant.ball_material)
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<OrchardCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> OrchardPlant {
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

	let (tree, world_size) = match placed.variant {
		OrchardCell::FruitingStorybook => OrchardFruiting::grow_num(variant),
		OrchardCell::PaleBloomStorybook => OrchardPaleBloom::grow_num(variant),
	};
	let placement = Placement::new(placed.position, 0.0)
		.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));

	OrchardPlant { placement, tree, stick_material, ball_material, frond_material }
}

crate::impl_woody_grove_lod!(Orchard, WOODY_LOD);

#[cfg(test)]
mod tests;
