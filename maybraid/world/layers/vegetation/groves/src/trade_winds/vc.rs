use super::WOODY_LOD;
use std::sync::Arc;

use super::variants::trade_winds_banyan::{HonuBanyanSamples, SopeBanyanSamples};

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
	HonuBanyan, QuantizedPlant, SopesBanyan, StorybookTree, StorybookTreeParams, WaialeaPalm,
	WaialeaPalmParams,
};
use vegetation_components::{FoliageNode, Placement, VegetationComponents};

use super::{
	definition, TradeWindsCell, TradeWindsItem, RARE_TALL_TRADE_STORYBOOK, RARE_TRADE_WAIALEA_PALM,
	TRADE_STORYBOOK,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, canopy_proxy_trunk, canopy_proxy_waialea,
	foliage_low_canopy_balls, frond_material_from_palette, grove_structural_footprint,
	lazy_flattened_plant_chunks, nest_flattened_plant_chunk, placed_palm_low_fronds,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TradeWindsParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TradeWindsCell>,
}

impl Default for TradeWindsParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.20 }),
		}
	}
}

crate::impl_grove_preview_params!(TradeWindsParams, TradeWindsCell);

impl TradeWindsParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TradeWinds {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TradeWinds {
		TradeWinds::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(TradeStorybook, StorybookTree, StorybookTreeParams, TRADE_STORYBOOK);
remixed_sbs_plant!(
	RareTallTradeStorybook,
	StorybookTree,
	StorybookTreeParams,
	RARE_TALL_TRADE_STORYBOOK
);
remixed_sbs_plant!(RareTradeWaialeaPalm, WaialeaPalm, WaialeaPalmParams, RARE_TRADE_WAIALEA_PALM);

#[derive(Clone)]
enum TradeWindsKind {
	Storybook(Arc<StorybookTree>),
	Honu(Arc<HonuBanyan>),
	Sope(Arc<SopesBanyan>),
	Waialea(Arc<WaialeaPalm>),
}

#[derive(Clone)]
pub struct TradeWindsPlant {
	pub placement: Placement,
	kind: TradeWindsKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TradeWinds {
	pub plants: Arc<[TradeWindsPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl TradeWinds {
	pub fn from_placements(
		placements: &[GroveCellVariant<TradeWindsCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[TradeWindsPlant]> = placements
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
				TradeWindsKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TradeWindsKind::Honu(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TradeWindsKind::Sope(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TradeWindsKind::Waialea(t) => nest_flattened_plant_chunk(
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
			.flat_map(|plant| {
				let material = &plant.ball_material;
				match &plant.kind {
					TradeWindsKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					TradeWindsKind::Honu(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					TradeWindsKind::Sope(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					TradeWindsKind::Waialea(t) => {
						canopy_proxy_waialea(t, plant.placement, &plant.stick_material, material)
					}
				}
			})
			.collect()
	}

	fn foliage_low_nodes(&self) -> Vec<FoliageNode> {
		let mut nodes = Vec::new();
		let mut sites = Vec::new();
		for plant in self.plants.iter() {
			let material = &plant.ball_material;
			match &plant.kind {
				TradeWindsKind::Storybook(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				TradeWindsKind::Honu(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				TradeWindsKind::Sope(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				TradeWindsKind::Waialea(t) => {
					nodes.extend(placed_palm_low_fronds(
						t.as_ref(),
						plant.placement,
						&plant.stick_material,
						material,
						&plant.frond_material,
					));
					if let Some(trunk) =
						canopy_proxy_trunk(t, plant.placement, &plant.stick_material)
					{
						sites.push(trunk);
					}
				}
			}
		}
		nodes.extend(foliage_low_canopy_balls(sites));
		nodes
	}
}

fn grow_plant(
	placed: &GroveCellVariant<TradeWindsCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> TradeWindsPlant {
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
		TradeWindsItem::Storybook(_) => {
			let (tree, world_size) = match placed.variant {
				TradeWindsCell::TradeStorybook => TradeStorybook::grow_num(variant),
				TradeWindsCell::RareTallTradeStorybook => RareTallTradeStorybook::grow_num(variant),
				_ => unreachable!("storybook item is only TradeStorybook cells"),
			};
			let placement = Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));
			TradeWindsPlant {
				placement,
				kind: TradeWindsKind::Storybook(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		TradeWindsItem::Honu(banyan) => {
			let world_size =
				BuildWithNoise::<HonuBanyanSamples>::build_with_noise(banyan, build_noise)
					.geometry
					.scale
					.tree_height;
			let placement = Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));
			TradeWindsPlant {
				placement,
				kind: TradeWindsKind::Honu(HonuBanyan::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		TradeWindsItem::Sope(banyan) => {
			let world_size =
				BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise)
					.geometry
					.scale
					.stalk_height;
			let placement = Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));
			TradeWindsPlant {
				placement,
				kind: TradeWindsKind::Sope(SopesBanyan::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		TradeWindsItem::WaialeaPalm(_) => {
			let (tree, world_size) = RareTradeWaialeaPalm::grow_num(variant);
			let placement = Placement::new(placed.position, 0.0)
				.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4)));
			TradeWindsPlant {
				placement,
				kind: TradeWindsKind::Waialea(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(TradeWinds, WOODY_LOD, low_nodes);

#[cfg(test)]
mod tests;
