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
use procedural_common::{BuildWithNoise, NoiseParams};
use sbs_trees::{
	QuantizedPlant, StorybookTree, StorybookTreeParams, TemperateConifer, TemperateConiferParams,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, LeewardCell, LeewardTemperateConifer, HIGH_LEEWARD_STORYBOOK,
	ROUNDED_LEEWARD_STORYBOOK, SHELTERED_TEMPERATE_CONIFER, WINDBREAK_TEMPERATE_CONIFER,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_column, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, placement_noise, remixed_sbs_plant, stick_material_from_palette,
	unit_build_noise, CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct LeewardParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<LeewardCell>,
}

impl Default for LeewardParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 }),
		}
	}
}

crate::impl_grove_preview_params!(LeewardParams, LeewardCell);

impl LeewardParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> Leeward {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Leeward {
		Leeward::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn leeward_temperate_unit(authored: &LeewardTemperateConifer, num: u32) -> (TemperateConifer, f32) {
	let samples = authored.build_with_noise(unit_build_noise(num));
	let mut params = TemperateConiferParams::default();
	params.geometry = samples.geometry.into();
	params.frond_world_scale = samples.frond_world_scale;
	params.fronds_per_joint = samples.fronds_per_joint;
	params.frond_length_fraction = samples.frond_length_fraction;
	params.frond_spawn_fraction = samples.frond_spawn_fraction;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct ShelteredTemperate;

impl QuantizedPlant for ShelteredTemperate {
	type Unit = TemperateConifer;

	fn build_unit(num: u32) -> (TemperateConifer, f32) {
		leeward_temperate_unit(&SHELTERED_TEMPERATE_CONIFER, num)
	}
}

struct WindbreakTemperate;

impl QuantizedPlant for WindbreakTemperate {
	type Unit = TemperateConifer;

	fn build_unit(num: u32) -> (TemperateConifer, f32) {
		leeward_temperate_unit(&WINDBREAK_TEMPERATE_CONIFER, num)
	}
}

remixed_sbs_plant!(
	RoundedLeewardStorybook,
	StorybookTree,
	StorybookTreeParams,
	ROUNDED_LEEWARD_STORYBOOK
);
remixed_sbs_plant!(
	HighLeewardStorybook,
	StorybookTree,
	StorybookTreeParams,
	HIGH_LEEWARD_STORYBOOK
);

#[derive(Clone)]
enum LeewardKind {
	Storybook(Arc<StorybookTree>),
	Temperate(Arc<TemperateConifer>),
}

#[derive(Clone)]
pub struct LeewardPlant {
	pub placement: Placement,
	kind: LeewardKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct Leeward {
	pub plants: Arc<[LeewardPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl Leeward {
	pub fn from_placements(
		placements: &[GroveCellVariant<LeewardCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[LeewardPlant]> = placements
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
			Some(match &plant.kind {
				LeewardKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LeewardKind::Temperate(t) => nest_flattened_plant_chunk(
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
			.filter_map(|plant| {
				let material = &plant.ball_material;
				match &plant.kind {
					LeewardKind::Storybook(t) => canopy_proxy_site(t, plant.placement, material),
					LeewardKind::Temperate(t) => canopy_proxy_column(t, plant.placement, material),
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<LeewardCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> LeewardPlant {
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

	let (kind, world_size) = match placed.variant {
		LeewardCell::ShelteredTemperateConifer => {
			let (tree, world_size) = ShelteredTemperate::grow_num(variant);
			(LeewardKind::Temperate(tree), world_size)
		}
		LeewardCell::WindbreakTemperateConifer => {
			let (tree, world_size) = WindbreakTemperate::grow_num(variant);
			(LeewardKind::Temperate(tree), world_size)
		}
		LeewardCell::RoundedLeewardStorybook => {
			let (tree, world_size) = RoundedLeewardStorybook::grow_num(variant);
			(LeewardKind::Storybook(tree), world_size)
		}
		LeewardCell::HighLeewardStorybook => {
			let (tree, world_size) = HighLeewardStorybook::grow_num(variant);
			(LeewardKind::Storybook(tree), world_size)
		}
	};

	LeewardPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(Leeward, WOODY_LOD);

#[cfg(test)]
mod tests;
