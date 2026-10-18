use super::WOODY_LOD;
use std::sync::Arc;

use super::variants::conifer_sapling_friends_conifer::FriendConiferSamples;

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
	FriendsConifer, FriendsConiferParams, NorthernConifer, NorthernConiferParams, QuantizedPlant,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, ConiferSaplingCell, ConiferSaplingFriendsConifer, ConiferSaplingNorthernConifer,
	BRIGHT_FRIEND_SAPLING, COLD_NORTHERN_SAPLING, FRIEND_SAPLING, MOSSY_FRIEND_SAPLING,
	NORTHERN_SAPLING, WINDSWEPT_NORTHERN_SAPLING,
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
pub struct ConiferSaplingParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<ConiferSaplingCell>,
}

impl Default for ConiferSaplingParams {
	fn default() -> Self {
		Self { preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()) }
	}
}

crate::impl_grove_preview_params!(ConiferSaplingParams, ConiferSaplingCell);

impl ConiferSaplingParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> ConiferSapling {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> ConiferSapling {
		ConiferSapling::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn friends_sapling_unit(
	authored: &ConiferSaplingFriendsConifer,
	num: u32,
) -> (FriendsConifer, f32) {
	let samples =
		BuildWithNoise::<FriendConiferSamples>::build_with_noise(authored, unit_build_noise(num));
	let mut params = FriendsConiferParams::default();
	params.geometry = samples.geometry;
	params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

fn northern_sapling_unit(
	authored: &ConiferSaplingNorthernConifer,
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

struct FriendSapling;
impl QuantizedPlant for FriendSapling {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		friends_sapling_unit(&FRIEND_SAPLING, num)
	}
}

struct MossyFriendSapling;
impl QuantizedPlant for MossyFriendSapling {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		friends_sapling_unit(&MOSSY_FRIEND_SAPLING, num)
	}
}

struct BrightFriendSapling;
impl QuantizedPlant for BrightFriendSapling {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		friends_sapling_unit(&BRIGHT_FRIEND_SAPLING, num)
	}
}

struct NorthernSapling;
impl QuantizedPlant for NorthernSapling {
	type Unit = NorthernConifer;
	fn build_unit(num: u32) -> (NorthernConifer, f32) {
		northern_sapling_unit(&NORTHERN_SAPLING, num)
	}
}

struct ColdNorthernSapling;
impl QuantizedPlant for ColdNorthernSapling {
	type Unit = NorthernConifer;
	fn build_unit(num: u32) -> (NorthernConifer, f32) {
		northern_sapling_unit(&COLD_NORTHERN_SAPLING, num)
	}
}

struct WindsweptNorthernSapling;
impl QuantizedPlant for WindsweptNorthernSapling {
	type Unit = NorthernConifer;
	fn build_unit(num: u32) -> (NorthernConifer, f32) {
		northern_sapling_unit(&WINDSWEPT_NORTHERN_SAPLING, num)
	}
}

#[derive(Clone)]
enum ConiferSaplingKind {
	Friends(Arc<FriendsConifer>),
	Northern(Arc<NorthernConifer>),
}

#[derive(Clone)]
pub struct ConiferSaplingPlant {
	pub placement: Placement,
	kind: ConiferSaplingKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct ConiferSapling {
	pub plants: Arc<[ConiferSaplingPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl ConiferSapling {
	pub fn from_placements(
		placements: &[GroveCellVariant<ConiferSaplingCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[ConiferSaplingPlant]> = placements
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
				ConiferSaplingKind::Friends(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ConiferSaplingKind::Northern(t) => nest_flattened_plant_chunk(
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
					ConiferSaplingKind::Friends(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					ConiferSaplingKind::Northern(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<ConiferSaplingCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> ConiferSaplingPlant {
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
		ConiferSaplingCell::FriendSapling => {
			let (tree, world_size) = FriendSapling::grow_num(variant);
			(ConiferSaplingKind::Friends(tree), world_size)
		}
		ConiferSaplingCell::MossyFriendSapling => {
			let (tree, world_size) = MossyFriendSapling::grow_num(variant);
			(ConiferSaplingKind::Friends(tree), world_size)
		}
		ConiferSaplingCell::BrightFriendSapling => {
			let (tree, world_size) = BrightFriendSapling::grow_num(variant);
			(ConiferSaplingKind::Friends(tree), world_size)
		}
		ConiferSaplingCell::NorthernSapling => {
			let (tree, world_size) = NorthernSapling::grow_num(variant);
			(ConiferSaplingKind::Northern(tree), world_size)
		}
		ConiferSaplingCell::ColdNorthernSapling => {
			let (tree, world_size) = ColdNorthernSapling::grow_num(variant);
			(ConiferSaplingKind::Northern(tree), world_size)
		}
		ConiferSaplingCell::WindsweptNorthernSapling => {
			let (tree, world_size) = WindsweptNorthernSapling::grow_num(variant);
			(ConiferSaplingKind::Northern(tree), world_size)
		}
	};

	ConiferSaplingPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(ConiferSapling, WOODY_LOD);

#[cfg(test)]
mod tests;
