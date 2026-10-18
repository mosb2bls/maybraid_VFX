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
use sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs};
use sbs_trees::{
	BraidOakTree, KamakuraTorch, KamakuraTorchParams, PenmarchTorch, PenmarchTorchParams,
	QuantizedPlant, StorybookTree, StorybookTreeParams,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, StorytellersCell, StorytellersItem, BLUE_FLAME_KAMAKURA, BLUE_MOON_STORYBOOK,
	BRIGHT_CANOPY_STORYBOOK, COLORFUL_STORYBOOK, FESTIVAL_TORCH_TREE, GOLDEN_LANTERN_PENMARCH,
	PINK_LANTERN_STORYBOOK, PURPLE_CROWN_STORYBOOK,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, unit_build_noise,
	CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct StorytellersParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<StorytellersCell>,
}

impl Default for StorytellersParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 }),
		}
	}
}

crate::impl_grove_preview_params!(StorytellersParams, StorytellersCell);

impl StorytellersParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> Storytellers {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Storytellers {
		Storytellers::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(ColorfulStorybook, StorybookTree, StorybookTreeParams, COLORFUL_STORYBOOK);
remixed_sbs_plant!(
	BrightCanopyStorybook,
	StorybookTree,
	StorybookTreeParams,
	BRIGHT_CANOPY_STORYBOOK
);
remixed_sbs_plant!(
	PinkLanternStorybook,
	StorybookTree,
	StorybookTreeParams,
	PINK_LANTERN_STORYBOOK
);
remixed_sbs_plant!(
	PurpleCrownStorybook,
	StorybookTree,
	StorybookTreeParams,
	PURPLE_CROWN_STORYBOOK
);
remixed_sbs_plant!(BlueMoonStorybook, StorybookTree, StorybookTreeParams, BLUE_MOON_STORYBOOK);

struct GoldenLanternPenmarch;
impl QuantizedPlant for GoldenLanternPenmarch {
	type Unit = PenmarchTorch;
	fn build_unit(num: u32) -> (PenmarchTorch, f32) {
		let mut params = PenmarchTorchParams::default();
		params.geometry = BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(
			&GOLDEN_LANTERN_PENMARCH,
			unit_build_noise(num),
		);
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}
}

struct FestivalTorchTree;
impl QuantizedPlant for FestivalTorchTree {
	type Unit = PenmarchTorch;
	fn build_unit(num: u32) -> (PenmarchTorch, f32) {
		let mut params = PenmarchTorchParams::default();
		params.geometry = BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(
			&FESTIVAL_TORCH_TREE,
			unit_build_noise(num),
		);
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}
}

struct BlueFlameKamakura;
impl QuantizedPlant for BlueFlameKamakura {
	type Unit = KamakuraTorch;
	fn build_unit(num: u32) -> (KamakuraTorch, f32) {
		let mut params = KamakuraTorchParams::default();
		params.geometry = BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(
			&BLUE_FLAME_KAMAKURA,
			unit_build_noise(num),
		);
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}
}

#[derive(Clone)]
enum StorytellersKind {
	Oak(Arc<BraidOakTree>),
	Storybook(Arc<StorybookTree>),
	Penmarch(Arc<PenmarchTorch>),
	Kamakura(Arc<KamakuraTorch>),
}

#[derive(Clone)]
pub struct StorytellersPlant {
	pub placement: Placement,
	kind: StorytellersKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct Storytellers {
	pub plants: Arc<[StorytellersPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl Storytellers {
	pub fn from_placements(
		placements: &[GroveCellVariant<StorytellersCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[StorytellersPlant]> = placements
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
				StorytellersKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				StorytellersKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				StorytellersKind::Penmarch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				StorytellersKind::Kamakura(t) => nest_flattened_plant_chunk(
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
					StorytellersKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
					StorytellersKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
					StorytellersKind::Penmarch(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
					StorytellersKind::Kamakura(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<StorytellersCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> StorytellersPlant {
	let variant = patch_variant_index(placed.position, tree_variants);
	let build_noise = variant_noise(grove_noise, variant);
	let palette_noise = placement_noise(grove_noise, placed.position);
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	let stick_material =
		stick_material_from_palette(Some(placed.variant.stick_palette_mix()), stick_seed);
	let ball_material =
		canopy_ball_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);
	let frond_material =
		frond_material_from_palette(Some(placed.variant.canopy_palette_mix()), canopy_seed);

	match placed.variant.item() {
		StorytellersItem::BraidOak(oak) => {
			let world_size = oak.build_with_noise(build_noise).height();
			StorytellersPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: StorytellersKind::Oak(BraidOakTree::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		StorytellersItem::Storybook(_) => {
			let (tree, world_size) = match placed.variant {
				StorytellersCell::ColorfulStorybook => ColorfulStorybook::grow_num(variant),
				StorytellersCell::BrightCanopyStorybook => BrightCanopyStorybook::grow_num(variant),
				StorytellersCell::PinkLanternStorybook => PinkLanternStorybook::grow_num(variant),
				StorytellersCell::PurpleCrownStorybook => PurpleCrownStorybook::grow_num(variant),
				StorytellersCell::BlueMoonStorybook => BlueMoonStorybook::grow_num(variant),
				_ => unreachable!("storybook item is only storybook cells"),
			};
			StorytellersPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: StorytellersKind::Storybook(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		StorytellersItem::PenmarchTorch(_) | StorytellersItem::TorchTree(_) => {
			let (tree, world_size) = match placed.variant {
				StorytellersCell::GoldenLanternPenmarch => GoldenLanternPenmarch::grow_num(variant),
				StorytellersCell::FestivalTorchTree => FestivalTorchTree::grow_num(variant),
				_ => unreachable!("penmarch item is only penmarch torch cells"),
			};
			StorytellersPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: StorytellersKind::Penmarch(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		StorytellersItem::KamakuraTorch(_) => {
			let (tree, world_size) = BlueFlameKamakura::grow_num(variant);
			StorytellersPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: StorytellersKind::Kamakura(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(Storytellers, WOODY_LOD);

#[cfg(test)]
mod tests;
