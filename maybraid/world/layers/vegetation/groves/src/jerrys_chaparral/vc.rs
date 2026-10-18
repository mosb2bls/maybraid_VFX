use super::WOODY_LOD;
use std::sync::Arc;

use super::variants::jerrys_chaparral_friends_conifer::ConiferSamples;

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
	FriendsConifer, FriendsConiferParams, HighBushShoots, QuantizedPlant, RorysHeadTrained,
	RorysHeadTrainedParams,
};
use vegetation_components::{Placement, StickNode, VegetationComponents};

use super::{
	definition, JerrysChaparralCell, JerrysChaparralFriendsConifer, CHAPARRAL_HIGH_BUSH,
	DRY_RORY_HEAD, MANZANITA_RORY, SMALL_FRIENDS_CONIFER,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, placement_noise, remixed_bush_plant, remixed_sbs_plant,
	stick_material_from_palette, unit_build_noise, CanopyProxySite, FlatTerrainSample,
	GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct JerrysChaparralParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<JerrysChaparralCell>,
}

impl Default for JerrysChaparralParams {
	fn default() -> Self {
		Self { preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()) }
	}
}

crate::impl_grove_preview_params!(JerrysChaparralParams, JerrysChaparralCell);

impl JerrysChaparralParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> JerrysChaparral {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> JerrysChaparral {
		JerrysChaparral::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(DryRoryHead, RorysHeadTrained, RorysHeadTrainedParams, DRY_RORY_HEAD);
remixed_sbs_plant!(ManzanitaRory, RorysHeadTrained, RorysHeadTrainedParams, MANZANITA_RORY);
remixed_bush_plant!(ChaparralHighBush, CHAPARRAL_HIGH_BUSH);

fn chaparral_friends_unit(
	authored: &JerrysChaparralFriendsConifer,
	num: u32,
) -> (FriendsConifer, f32) {
	let samples =
		BuildWithNoise::<ConiferSamples>::build_with_noise(authored, unit_build_noise(num));
	let mut params = FriendsConiferParams::default();
	params.geometry = samples.geometry;
	params.splay_radius_fraction_of_height = samples.splay_radius_fraction_of_height;
	params.apex_canopy_spawn_fraction = samples.apex_canopy_spawn_fraction;
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct SmallFriendsConifer;

impl QuantizedPlant for SmallFriendsConifer {
	type Unit = FriendsConifer;
	fn build_unit(num: u32) -> (FriendsConifer, f32) {
		chaparral_friends_unit(&SMALL_FRIENDS_CONIFER, num)
	}
}

#[derive(Clone)]
enum JerrysChaparralKind {
	Rory(Arc<RorysHeadTrained>),
	Bush(Arc<HighBushShoots>),
	Friends(Arc<FriendsConifer>),
}

#[derive(Clone)]
pub struct JerrysChaparralPlant {
	pub placement: Placement,
	kind: JerrysChaparralKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct JerrysChaparral {
	pub plants: Arc<[JerrysChaparralPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl JerrysChaparral {
	pub fn from_placements(
		placements: &[GroveCellVariant<JerrysChaparralCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[JerrysChaparralPlant]> = placements
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
				JerrysChaparralKind::Rory(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				JerrysChaparralKind::Bush(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				JerrysChaparralKind::Friends(t) => nest_flattened_plant_chunk(
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
					JerrysChaparralKind::Rory(t) => vec![
						canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
							.crown,
					],
					JerrysChaparralKind::Bush(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					JerrysChaparralKind::Friends(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
				}
			})
			.collect()
	}

	fn proxy_trunks(&self) -> Vec<StickNode> {
		self.plants
			.iter()
			.filter_map(|plant| match &plant.kind {
				JerrysChaparralKind::Rory(t) => {
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

fn grow_plant(
	placed: &GroveCellVariant<JerrysChaparralCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> JerrysChaparralPlant {
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
		JerrysChaparralCell::DryRoryHeadTrained => {
			let (tree, world_size) = DryRoryHead::grow_num(variant);
			(JerrysChaparralKind::Rory(tree), world_size)
		}
		JerrysChaparralCell::ManzanitaRory => {
			let (tree, world_size) = ManzanitaRory::grow_num(variant);
			(JerrysChaparralKind::Rory(tree), world_size)
		}
		JerrysChaparralCell::ChaparralHighBush => {
			let (tree, world_size) = ChaparralHighBush::grow_num(variant);
			(JerrysChaparralKind::Bush(tree), world_size)
		}
		JerrysChaparralCell::SmallFriendsConifer => {
			let (tree, world_size) = SmallFriendsConifer::grow_num(variant);
			(JerrysChaparralKind::Friends(tree), world_size)
		}
	};

	JerrysChaparralPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(JerrysChaparral, WOODY_LOD, trunks);

#[cfg(test)]
mod tests;
