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
	BraidOakTree, FriendsConifer, FriendsConiferParams, QuantizedPlant, StorybookTree,
	StorybookTreeParams, TemperateConifer, TemperateConiferParams,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, RiparianMixCell, RiparianMixItem, BANK_FRIEND_CONIFER, ROUND_RIPARIAN_STORYBOOK,
	SHELTERED_TEMPERATE_CONIFER, TALL_RIPARIAN_STORYBOOK,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_column, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, placement_noise, remixed_sbs_plant, stick_material_from_palette,
	unit_build_noise, CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct RiparianMixParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<RiparianMixCell>,
}

impl Default for RiparianMixParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(RiparianMixParams, RiparianMixCell);

impl RiparianMixParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> RiparianMix {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> RiparianMix {
		RiparianMix::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(
	RoundRiparianStorybook,
	StorybookTree,
	StorybookTreeParams,
	ROUND_RIPARIAN_STORYBOOK
);
remixed_sbs_plant!(
	TallRiparianStorybook,
	StorybookTree,
	StorybookTreeParams,
	TALL_RIPARIAN_STORYBOOK
);

struct BankFriendConifer;
impl QuantizedPlant for BankFriendConifer {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		let samples = BANK_FRIEND_CONIFER.build_with_noise(unit_build_noise(num));
		let mut params = FriendsConiferParams::default();
		params.geometry = samples.geometry;
		params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
		params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
		let (unit, world_size) = params.into_unit_from_num(num);
		(unit.build(), world_size)
	}
}

struct ShelteredTemperateConifer;
impl QuantizedPlant for ShelteredTemperateConifer {
	type Unit = TemperateConifer;
	fn build_unit(num: u32) -> (TemperateConifer, f32) {
		let samples = SHELTERED_TEMPERATE_CONIFER.build_with_noise(unit_build_noise(num));
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
}

#[derive(Clone)]
enum RiparianMixKind {
	Oak(Arc<BraidOakTree>),
	Storybook(Arc<StorybookTree>),
	Friends(Arc<FriendsConifer>),
	Temperate(Arc<TemperateConifer>),
}

#[derive(Clone)]
pub struct RiparianMixPlant {
	pub placement: Placement,
	kind: RiparianMixKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct RiparianMix {
	pub plants: Arc<[RiparianMixPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl RiparianMix {
	pub fn from_placements(
		placements: &[GroveCellVariant<RiparianMixCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[RiparianMixPlant]> = placements
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
				RiparianMixKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				RiparianMixKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				RiparianMixKind::Friends(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				RiparianMixKind::Temperate(t) => nest_flattened_plant_chunk(
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
					RiparianMixKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
					RiparianMixKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
					RiparianMixKind::Friends(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					RiparianMixKind::Temperate(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<RiparianMixCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> RiparianMixPlant {
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
		RiparianMixItem::BraidOak(oak) => {
			let world_size = oak.build_with_noise(build_noise).height();
			RiparianMixPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: RiparianMixKind::Oak(BraidOakTree::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		RiparianMixItem::Storybook(_) => {
			let (tree, world_size) = match placed.variant {
				RiparianMixCell::RoundRiparianStorybook => {
					RoundRiparianStorybook::grow_num(variant)
				}
				RiparianMixCell::TallRiparianStorybook => TallRiparianStorybook::grow_num(variant),
				_ => unreachable!("storybook item is only riparian storybook cells"),
			};
			RiparianMixPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: RiparianMixKind::Storybook(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		RiparianMixItem::FriendsConifer(_) => {
			let (tree, world_size) = BankFriendConifer::grow_num(variant);
			RiparianMixPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: RiparianMixKind::Friends(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		RiparianMixItem::TemperateConifer(_) => {
			let (tree, world_size) = ShelteredTemperateConifer::grow_num(variant);
			RiparianMixPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: RiparianMixKind::Temperate(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(RiparianMix, WOODY_LOD);

#[cfg(test)]
mod tests;
