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
use procedural_common::{noise_params_from_scalar_str, BuildWithNoise, NoiseParams};
use sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs};
use sbs_trees::{
	KamakuraTorch, KamakuraTorchParams, PalmBush, PalmBushParams, PenmarchTorch,
	PenmarchTorchParams, QuantizedPlant, RorysHeadTrained, RorysHeadTrainedParams, StorybookTree,
	StorybookTreeParams, TuftPatch, VaseTree, VaseTreeParams,
};
use vegetation_components::{Placement, StickNode, VegetationComponents};

use super::{
	definition, TropicalUndergrowthCell, TropicalUndergrowthTorch, BRIGHT_TUFT, BRIGHT_TUFT_PATCH,
	DEEP_TUFT, DEEP_TUFT_PATCH, MINI_KAMAKURA_TORCH, MINI_PENMARCH_TORCH, MINI_RORY_HEAD,
	MINI_STORYBOOK, MINI_TORCH_TREE, MINI_VASE_TREE, SMALL_PALM_BUSH,
};
use crate::grove::vc_tuft::{material_from_palette, patch_variant_index, unit_plant_from_grown};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, placement_noise, remixed_blade_tuft_plant, remixed_sbs_plant,
	remixed_tuft_plant, stick_material_from_palette, unit_build_noise, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalUndergrowthParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TropicalUndergrowthCell>,

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

impl Default for TropicalUndergrowthParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(TropicalUndergrowthParams, TropicalUndergrowthCell);

impl TropicalUndergrowthParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TropicalUndergrowth {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalUndergrowth {
		TropicalUndergrowth::from_placements(
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

remixed_blade_tuft_plant!(BrightTuft, BRIGHT_TUFT, default_foliage());
remixed_blade_tuft_plant!(DeepTuft, DEEP_TUFT, default_foliage());
remixed_tuft_plant!(BrightTuftPatch, BRIGHT_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(DeepTuftPatch, DEEP_TUFT_PATCH, default_foliage());
remixed_sbs_plant!(SmallPalmBush, PalmBush, PalmBushParams, SMALL_PALM_BUSH);
remixed_sbs_plant!(MiniRoryHead, RorysHeadTrained, RorysHeadTrainedParams, MINI_RORY_HEAD);
remixed_sbs_plant!(MiniVaseTree, VaseTree, VaseTreeParams, MINI_VASE_TREE);
remixed_sbs_plant!(MiniSparseStorybook, StorybookTree, StorybookTreeParams, MINI_STORYBOOK);

fn undergrowth_penmarch_unit(
	authored: &TropicalUndergrowthTorch,
	num: u32,
) -> (PenmarchTorch, f32) {
	let mut params = PenmarchTorchParams::default();
	params.geometry =
		BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(authored, unit_build_noise(num));
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

fn undergrowth_kamakura_unit(
	authored: &TropicalUndergrowthTorch,
	num: u32,
) -> (KamakuraTorch, f32) {
	let mut params = KamakuraTorchParams::default();
	params.geometry =
		BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(authored, unit_build_noise(num));
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct MiniPenmarchTorch;
struct MiniKamakuraTorch;
struct MiniTorchTree;

impl QuantizedPlant for MiniPenmarchTorch {
	type Unit = PenmarchTorch;
	fn build_unit(num: u32) -> (PenmarchTorch, f32) {
		undergrowth_penmarch_unit(&MINI_PENMARCH_TORCH, num)
	}
}

impl QuantizedPlant for MiniKamakuraTorch {
	type Unit = KamakuraTorch;
	fn build_unit(num: u32) -> (KamakuraTorch, f32) {
		undergrowth_kamakura_unit(&MINI_KAMAKURA_TORCH, num)
	}
}

impl QuantizedPlant for MiniTorchTree {
	type Unit = PenmarchTorch;
	fn build_unit(num: u32) -> (PenmarchTorch, f32) {
		undergrowth_penmarch_unit(&MINI_TORCH_TREE, num)
	}
}

#[derive(Clone)]
enum TropicalUndergrowthKind {
	Tuft(Arc<TuftPatch>),
	Palm(Arc<PalmBush>),
	Rory(Arc<RorysHeadTrained>),
	Vase(Arc<VaseTree>),
	Storybook(Arc<StorybookTree>),
	Penmarch(Arc<PenmarchTorch>),
	Kamakura(Arc<KamakuraTorch>),
}

#[derive(Clone)]
struct TropicalUndergrowthPlant {
	placement: Placement,
	kind: TropicalUndergrowthKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TropicalUndergrowth {
	plants: Arc<[TropicalUndergrowthPlant]>,
	structural_center: Vec3,
	footprint_radius: f32,
	pub extent: GroveExtent,
}

impl TropicalUndergrowth {
	pub fn from_placements(
		placements: &[GroveCellVariant<TropicalUndergrowthCell>],
		grove_noise: NoiseParams,
		leaf_surface_noise: NoiseParams,
		patch_variants: u32,
		tree_variants: u32,
		extent: &GroveExtent,
	) -> Self {
		let patch_variants = patch_variants.max(1);
		let tree_variants = tree_variants.max(1);
		let plants: Arc<[TropicalUndergrowthPlant]> = placements
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
				TropicalUndergrowthKind::Tuft(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Palm(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Rory(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Vase(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Penmarch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalUndergrowthKind::Kamakura(t) => nest_flattened_plant_chunk(
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
			.flat_map(|plant| match &plant.kind {
				TropicalUndergrowthKind::Tuft(t) => {
					vec![tuft_proxy_site(t, plant.placement, &plant.ball_material)]
				}
				TropicalUndergrowthKind::Palm(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
				TropicalUndergrowthKind::Rory(t) => vec![
					canopy_proxy_rory(
						t,
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
					)
					.crown,
				],
				TropicalUndergrowthKind::Vase(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
				TropicalUndergrowthKind::Storybook(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
				TropicalUndergrowthKind::Penmarch(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
				TropicalUndergrowthKind::Kamakura(t) => {
					canopy_proxy_site(t, plant.placement, &plant.ball_material)
						.into_iter()
						.collect()
				}
			})
			.collect()
	}

	fn proxy_trunks(&self) -> Vec<StickNode> {
		self.plants
			.iter()
			.filter_map(|plant| match &plant.kind {
				TropicalUndergrowthKind::Rory(t) => {
					canopy_proxy_rory(
						t,
						plant.placement,
						&plant.stick_material,
						&plant.ball_material,
					)
					.trunk
				}
				_ => None,
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

fn woody_materials(
	placed: &GroveCellVariant<TropicalUndergrowthCell>,
	palette_noise: NoiseParams,
) -> (MaterialRef, MaterialRef, MaterialRef) {
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	(
		stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed),
		canopy_ball_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed),
		frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed),
	)
}

fn grow_plant(
	placed: &GroveCellVariant<TropicalUndergrowthCell>,
	grove_noise: NoiseParams,
	leaf_surface_noise: NoiseParams,
	patch_variants: u32,
	tree_variants: u32,
) -> TropicalUndergrowthPlant {
	match placed.variant {
		TropicalUndergrowthCell::BrightTuft
		| TropicalUndergrowthCell::DeepTuft
		| TropicalUndergrowthCell::BrightTuftPatch
		| TropicalUndergrowthCell::DeepTuftPatch => {
			let variant = patch_variant_index(placed.position, patch_variants);
			let (patch, world_size) = match placed.variant {
				TropicalUndergrowthCell::BrightTuft => BrightTuft::grow_num(variant),
				TropicalUndergrowthCell::DeepTuft => DeepTuft::grow_num(variant),
				TropicalUndergrowthCell::BrightTuftPatch => BrightTuftPatch::grow_num(variant),
				TropicalUndergrowthCell::DeepTuftPatch => DeepTuftPatch::grow_num(variant),
				_ => unreachable!("tuft cells only"),
			};
			let material = material_from_palette(
				placed.variant.palette_mix(),
				placed.position,
				leaf_surface_noise,
			);
			let (placement, patch, material) =
				unit_plant_from_grown(patch, world_size, placed.position, placed.scale, material);
			TropicalUndergrowthPlant {
				placement,
				kind: TropicalUndergrowthKind::Tuft(patch),
				stick_material: MaterialRef::default(),
				ball_material: material.clone(),
				frond_material: material,
			}
		}
		TropicalUndergrowthCell::SmallPalmBush
		| TropicalUndergrowthCell::MiniRoryHeadTrained
		| TropicalUndergrowthCell::MiniVaseTree
		| TropicalUndergrowthCell::MiniSparseStorybook
		| TropicalUndergrowthCell::MiniPenmarchTorch
		| TropicalUndergrowthCell::MiniKamakuraTorch
		| TropicalUndergrowthCell::MiniTorchTree => {
			let variant = patch_variant_index(placed.position, tree_variants);
			let palette_noise = placement_noise(grove_noise, placed.position);
			let (stick, ball, frond) = woody_materials(placed, palette_noise);
			let (kind, world_size) = match placed.variant {
				TropicalUndergrowthCell::SmallPalmBush => {
					let (tree, world_size) = SmallPalmBush::grow_num(variant);
					(TropicalUndergrowthKind::Palm(tree), world_size)
				}
				TropicalUndergrowthCell::MiniRoryHeadTrained => {
					let (tree, world_size) = MiniRoryHead::grow_num(variant);
					(TropicalUndergrowthKind::Rory(tree), world_size)
				}
				TropicalUndergrowthCell::MiniVaseTree => {
					let (tree, world_size) = MiniVaseTree::grow_num(variant);
					(TropicalUndergrowthKind::Vase(tree), world_size)
				}
				TropicalUndergrowthCell::MiniSparseStorybook => {
					let (tree, world_size) = MiniSparseStorybook::grow_num(variant);
					(TropicalUndergrowthKind::Storybook(tree), world_size)
				}
				TropicalUndergrowthCell::MiniPenmarchTorch => {
					let (tree, world_size) = MiniPenmarchTorch::grow_num(variant);
					(TropicalUndergrowthKind::Penmarch(tree), world_size)
				}
				TropicalUndergrowthCell::MiniKamakuraTorch => {
					let (tree, world_size) = MiniKamakuraTorch::grow_num(variant);
					(TropicalUndergrowthKind::Kamakura(tree), world_size)
				}
				TropicalUndergrowthCell::MiniTorchTree => {
					let (tree, world_size) = MiniTorchTree::grow_num(variant);
					(TropicalUndergrowthKind::Penmarch(tree), world_size)
				}
				TropicalUndergrowthCell::BrightTuft
				| TropicalUndergrowthCell::DeepTuft
				| TropicalUndergrowthCell::BrightTuftPatch
				| TropicalUndergrowthCell::DeepTuftPatch => {
					unreachable!("tuft cells are handled above")
				}
			};
			TropicalUndergrowthPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind,
				stick_material: stick,
				ball_material: ball,
				frond_material: frond,
			}
		}
	}
}

crate::impl_woody_grove_lod!(TropicalUndergrowth, WOODY_LOD, trunks);

#[cfg(test)]
mod tests;
