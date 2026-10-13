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
use sbs_trees::{
	FriendsConifer, FriendsConiferParams, LiamsConifer, LiamsConiferParams, NorthernConifer,
	NorthernConiferParams, QuantizedPlant,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, AridConiferSaplingCell, BARE_DRY_FRIEND_SAPLING, BARE_DRY_NORTHERN_SAPLING,
	DRY_FRIEND_SAPLING, DRY_LIAMS_SAPLING, DRY_NORTHERN_SAPLING, WISPY_DRY_FRIEND_SAPLING,
	WISPY_DRY_NORTHERN_SAPLING,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_column, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct AridConiferSaplingParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<AridConiferSaplingCell>,
}

impl Default for AridConiferSaplingParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(AridConiferSaplingParams, AridConiferSaplingCell);

impl AridConiferSaplingParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> AridConiferSapling {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> AridConiferSapling {
		AridConiferSapling::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(DryFriendSapling, FriendsConifer, FriendsConiferParams, DRY_FRIEND_SAPLING);
remixed_sbs_plant!(
	DryNorthernSapling,
	NorthernConifer,
	NorthernConiferParams,
	DRY_NORTHERN_SAPLING
);
remixed_sbs_plant!(
	WispyDryFriendSapling,
	FriendsConifer,
	FriendsConiferParams,
	WISPY_DRY_FRIEND_SAPLING
);
remixed_sbs_plant!(
	WispyDryNorthernSapling,
	NorthernConifer,
	NorthernConiferParams,
	WISPY_DRY_NORTHERN_SAPLING
);
remixed_sbs_plant!(
	BareDryFriendSapling,
	FriendsConifer,
	FriendsConiferParams,
	BARE_DRY_FRIEND_SAPLING
);
remixed_sbs_plant!(
	BareDryNorthernSapling,
	NorthernConifer,
	NorthernConiferParams,
	BARE_DRY_NORTHERN_SAPLING
);
remixed_sbs_plant!(DryLiamsConiferSapling, LiamsConifer, LiamsConiferParams, DRY_LIAMS_SAPLING);

#[derive(Clone)]
enum AridConiferSaplingKind {
	Friends(Arc<FriendsConifer>),
	Northern(Arc<NorthernConifer>),
	Liams(Arc<LiamsConifer>),
}

#[derive(Clone)]
pub struct AridConiferSaplingPlant {
	pub placement: Placement,
	kind: AridConiferSaplingKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct AridConiferSapling {
	pub plants: Arc<[AridConiferSaplingPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl AridConiferSapling {
	pub fn from_placements(
		placements: &[GroveCellVariant<AridConiferSaplingCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[AridConiferSaplingPlant]> = placements
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
				AridConiferSaplingKind::Friends(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				AridConiferSaplingKind::Northern(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				AridConiferSaplingKind::Liams(t) => nest_flattened_plant_chunk(
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
					AridConiferSaplingKind::Friends(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					AridConiferSaplingKind::Northern(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					AridConiferSaplingKind::Liams(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<AridConiferSaplingCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> AridConiferSaplingPlant {
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
		AridConiferSaplingCell::DryFriendSapling => {
			let (tree, world_size) = DryFriendSapling::grow_num(variant);
			(AridConiferSaplingKind::Friends(tree), world_size)
		}
		AridConiferSaplingCell::DryNorthernSapling => {
			let (tree, world_size) = DryNorthernSapling::grow_num(variant);
			(AridConiferSaplingKind::Northern(tree), world_size)
		}
		AridConiferSaplingCell::WispyDryFriendSapling => {
			let (tree, world_size) = WispyDryFriendSapling::grow_num(variant);
			(AridConiferSaplingKind::Friends(tree), world_size)
		}
		AridConiferSaplingCell::WispyDryNorthernSapling => {
			let (tree, world_size) = WispyDryNorthernSapling::grow_num(variant);
			(AridConiferSaplingKind::Northern(tree), world_size)
		}
		AridConiferSaplingCell::BareDryFriendSapling => {
			let (tree, world_size) = BareDryFriendSapling::grow_num(variant);
			(AridConiferSaplingKind::Friends(tree), world_size)
		}
		AridConiferSaplingCell::BareDryNorthernSapling => {
			let (tree, world_size) = BareDryNorthernSapling::grow_num(variant);
			(AridConiferSaplingKind::Northern(tree), world_size)
		}
		AridConiferSaplingCell::DryLiamsConiferSapling => {
			let (tree, world_size) = DryLiamsConiferSapling::grow_num(variant);
			(AridConiferSaplingKind::Liams(tree), world_size)
		}
	};

	AridConiferSaplingPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(AridConiferSapling, WOODY_LOD);

#[cfg(test)]
mod tests;
