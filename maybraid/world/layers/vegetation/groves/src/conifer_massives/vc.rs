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
	FriendsConifer, FriendsConiferParams, LiamsConifer, LiamsConiferParams, NorthernConifer,
	NorthernConiferParams, QuantizedPlant, TemperateConifer, TemperateConiferParams,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, ConiferMassivesCell, ConiferMassivesFriendsConifer, ConiferMassivesNorthernConifer,
	ConiferMassivesTemperateConifer, MASSIVE_FRIENDS_CONIFER, MASSIVE_LIAMS_CONIFER,
	MASSIVE_NORTHERN_CONIFER, MASSIVE_TEMPERATE_CONIFER,
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
pub struct ConiferMassivesParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<ConiferMassivesCell>,
}

impl Default for ConiferMassivesParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(ConiferMassivesParams, ConiferMassivesCell);

impl ConiferMassivesParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> ConiferMassives {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> ConiferMassives {
		ConiferMassives::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn massive_northern_unit(
	authored: &ConiferMassivesNorthernConifer,
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

fn massive_friends_unit(
	authored: &ConiferMassivesFriendsConifer,
	num: u32,
) -> (FriendsConifer, f32) {
	let samples = authored.build_with_noise(unit_build_noise(num));
	let mut params = FriendsConiferParams::default();
	params.geometry = samples.geometry;
	params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

fn massive_temperate_unit(
	authored: &ConiferMassivesTemperateConifer,
	num: u32,
) -> (TemperateConifer, f32) {
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

struct MassiveNorthernConifer;
impl QuantizedPlant for MassiveNorthernConifer {
	type Unit = NorthernConifer;
	fn build_unit(num: u32) -> (NorthernConifer, f32) {
		massive_northern_unit(&MASSIVE_NORTHERN_CONIFER, num)
	}
}

struct MassiveFriendsConifer;
impl QuantizedPlant for MassiveFriendsConifer {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		massive_friends_unit(&MASSIVE_FRIENDS_CONIFER, num)
	}
}

remixed_sbs_plant!(MassiveLiamsConifer, LiamsConifer, LiamsConiferParams, MASSIVE_LIAMS_CONIFER);

struct MassiveTemperateConifer;
impl QuantizedPlant for MassiveTemperateConifer {
	type Unit = TemperateConifer;
	fn build_unit(num: u32) -> (TemperateConifer, f32) {
		massive_temperate_unit(&MASSIVE_TEMPERATE_CONIFER, num)
	}
}

#[derive(Clone)]
enum ConiferMassivesKind {
	Northern(Arc<NorthernConifer>),
	Friends(Arc<FriendsConifer>),
	Liams(Arc<LiamsConifer>),
	Temperate(Arc<TemperateConifer>),
}

#[derive(Clone)]
pub struct ConiferMassivesPlant {
	pub placement: Placement,
	kind: ConiferMassivesKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct ConiferMassives {
	pub plants: Arc<[ConiferMassivesPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl ConiferMassives {
	pub fn from_placements(
		placements: &[GroveCellVariant<ConiferMassivesCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[ConiferMassivesPlant]> = placements
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
				ConiferMassivesKind::Northern(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ConiferMassivesKind::Friends(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ConiferMassivesKind::Liams(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ConiferMassivesKind::Temperate(t) => nest_flattened_plant_chunk(
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
					ConiferMassivesKind::Northern(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					ConiferMassivesKind::Friends(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					ConiferMassivesKind::Liams(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
					ConiferMassivesKind::Temperate(t) => {
						canopy_proxy_column(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<ConiferMassivesCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> ConiferMassivesPlant {
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
		ConiferMassivesCell::MassiveNorthernConifer => {
			let (tree, world_size) = MassiveNorthernConifer::grow_num(variant);
			(ConiferMassivesKind::Northern(tree), world_size)
		}
		ConiferMassivesCell::MassiveFriendsConifer => {
			let (tree, world_size) = MassiveFriendsConifer::grow_num(variant);
			(ConiferMassivesKind::Friends(tree), world_size)
		}
		ConiferMassivesCell::MassiveLiamsConifer => {
			let (tree, world_size) = MassiveLiamsConifer::grow_num(variant);
			(ConiferMassivesKind::Liams(tree), world_size)
		}
		ConiferMassivesCell::MassiveTemperateConifer => {
			let (tree, world_size) = MassiveTemperateConifer::grow_num(variant);
			(ConiferMassivesKind::Temperate(tree), world_size)
		}
	};

	ConiferMassivesPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(ConiferMassives, WOODY_LOD);

#[cfg(test)]
mod tests;
