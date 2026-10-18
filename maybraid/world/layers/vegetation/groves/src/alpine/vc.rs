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
	FriendsConifer, FriendsConiferParams, LiamsConifer, LiamsConiferParams, QuantizedPlant,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, AlpineCell, AlpineFriendsConifer, ALPINE_LIAMS, NEEDLE_SPIRE_LIAMS,
	TALL_ALPINE_FRIENDS, WINDLINE_FRIENDS,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_column, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, unit_build_noise,
	CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct AlpineParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<AlpineCell>,
}

impl Default for AlpineParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(AlpineParams, AlpineCell);

impl AlpineParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> Alpine {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Alpine {
		Alpine::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn alpine_friends_unit(authored: &AlpineFriendsConifer, num: u32) -> (FriendsConifer, f32) {
	let samples = authored.build_with_noise(unit_build_noise(num));
	let mut params = FriendsConiferParams::default();
	params.geometry = samples.geometry;
	params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct TallAlpineFriends;

impl QuantizedPlant for TallAlpineFriends {
	type Unit = FriendsConifer;

	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		alpine_friends_unit(&TALL_ALPINE_FRIENDS, num)
	}
}

struct WindlineFriends;

impl QuantizedPlant for WindlineFriends {
	type Unit = FriendsConifer;

	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		alpine_friends_unit(&WINDLINE_FRIENDS, num)
	}
}

remixed_sbs_plant!(AlpineLiams, LiamsConifer, LiamsConiferParams, ALPINE_LIAMS);
remixed_sbs_plant!(NeedleSpireLiams, LiamsConifer, LiamsConiferParams, NEEDLE_SPIRE_LIAMS);

#[derive(Clone)]
enum AlpineKind {
	Friends(Arc<FriendsConifer>),
	Liams(Arc<LiamsConifer>),
}

#[derive(Clone)]
pub struct AlpinePlant {
	pub placement: Placement,
	kind: AlpineKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct Alpine {
	pub plants: Arc<[AlpinePlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl Alpine {
	pub fn from_placements(
		placements: &[GroveCellVariant<AlpineCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[AlpinePlant]> = placements
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
				AlpineKind::Friends(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				AlpineKind::Liams(t) => nest_flattened_plant_chunk(
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
					AlpineKind::Friends(t) => canopy_proxy_column(t, plant.placement, material),
					AlpineKind::Liams(t) => canopy_proxy_column(t, plant.placement, material),
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<AlpineCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> AlpinePlant {
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
		AlpineCell::TallAlpineFriendsConifer => {
			let (tree, world_size) = TallAlpineFriends::grow_num(variant);
			(AlpineKind::Friends(tree), world_size)
		}
		AlpineCell::WindlineFriendsConifer => {
			let (tree, world_size) = WindlineFriends::grow_num(variant);
			(AlpineKind::Friends(tree), world_size)
		}
		AlpineCell::AlpineLiamsConifer => {
			let (tree, world_size) = AlpineLiams::grow_num(variant);
			(AlpineKind::Liams(tree), world_size)
		}
		AlpineCell::NeedleSpireLiamsConifer => {
			let (tree, world_size) = NeedleSpireLiams::grow_num(variant);
			(AlpineKind::Liams(tree), world_size)
		}
	};

	AlpinePlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(Alpine, WOODY_LOD);

#[cfg(test)]
mod tests;
