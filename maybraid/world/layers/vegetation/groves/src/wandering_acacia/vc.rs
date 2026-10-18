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
	HighBushShoots, KamakuraTorch, KamakuraTorchParams, PenmarchTorch, PenmarchTorchParams,
	QuantizedPlant, SopesBanyan, VaseTree, VaseTreeParams,
};
use vegetation_components::{Placement, VegetationComponents};

use super::{
	definition, WanderingAcaciaCell, WanderingAcaciaTorch, DRY_WANDERING_SOPE, WANDERING_HIGH_BUSH,
	WANDERING_KAMAKURA_TORCH, WANDERING_PENMARCH_TORCH, WANDERING_VASE_TREE,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_bush_plant, remixed_sbs_plant, stick_material_from_palette,
	unit_build_noise, CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent,
	GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct WanderingAcaciaParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<WanderingAcaciaCell>,
}

impl Default for WanderingAcaciaParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.40, steepness: 0.25 }),
		}
	}
}

crate::impl_grove_preview_params!(WanderingAcaciaParams, WanderingAcaciaCell);

impl WanderingAcaciaParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> WanderingAcacia {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> WanderingAcacia {
		WanderingAcacia::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_bush_plant!(WanderingHighBush, WANDERING_HIGH_BUSH);
remixed_sbs_plant!(WanderingVaseTree, VaseTree, VaseTreeParams, WANDERING_VASE_TREE);

fn wandering_penmarch_unit(authored: &WanderingAcaciaTorch, num: u32) -> (PenmarchTorch, f32) {
	let mut params = PenmarchTorchParams::default();
	params.geometry =
		BuildWithNoise::<PenmarchTorchSbs>::build_with_noise(authored, unit_build_noise(num));
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

fn wandering_kamakura_unit(authored: &WanderingAcaciaTorch, num: u32) -> (KamakuraTorch, f32) {
	let mut params = KamakuraTorchParams::default();
	params.geometry =
		BuildWithNoise::<KamakuraTorchSbs>::build_with_noise(authored, unit_build_noise(num));
	let (unit, world_size) = params.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct WanderingPenmarchTorch;
struct WanderingKamakuraTorch;

impl QuantizedPlant for WanderingPenmarchTorch {
	type Unit = PenmarchTorch;
	fn build_unit(num: u32) -> (PenmarchTorch, f32) {
		wandering_penmarch_unit(&WANDERING_PENMARCH_TORCH, num)
	}
}

impl QuantizedPlant for WanderingKamakuraTorch {
	type Unit = KamakuraTorch;
	fn build_unit(num: u32) -> (KamakuraTorch, f32) {
		wandering_kamakura_unit(&WANDERING_KAMAKURA_TORCH, num)
	}
}

#[derive(Clone)]
enum WanderingAcaciaKind {
	Bush(Arc<HighBushShoots>),
	Sope(Arc<SopesBanyan>),
	Vase(Arc<VaseTree>),
	Penmarch(Arc<PenmarchTorch>),
	Kamakura(Arc<KamakuraTorch>),
}

#[derive(Clone)]
pub struct WanderingAcaciaPlant {
	pub placement: Placement,
	kind: WanderingAcaciaKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct WanderingAcacia {
	pub plants: Arc<[WanderingAcaciaPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl WanderingAcacia {
	pub fn from_placements(
		placements: &[GroveCellVariant<WanderingAcaciaCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[WanderingAcaciaPlant]> = placements
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
				WanderingAcaciaKind::Bush(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				WanderingAcaciaKind::Sope(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				WanderingAcaciaKind::Vase(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				WanderingAcaciaKind::Penmarch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				WanderingAcaciaKind::Kamakura(t) => nest_flattened_plant_chunk(
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
					WanderingAcaciaKind::Bush(t) => canopy_proxy_site(t, plant.placement, material),
					WanderingAcaciaKind::Sope(t) => canopy_proxy_site(t, plant.placement, material),
					WanderingAcaciaKind::Vase(t) => canopy_proxy_site(t, plant.placement, material),
					WanderingAcaciaKind::Penmarch(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
					WanderingAcaciaKind::Kamakura(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<WanderingAcaciaCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> WanderingAcaciaPlant {
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
		WanderingAcaciaCell::WanderingHighBush => {
			let (tree, world_size) = WanderingHighBush::grow_num(variant);
			(WanderingAcaciaKind::Bush(tree), world_size)
		}
		WanderingAcaciaCell::DryWanderingSopesBanyan => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size =
				DRY_WANDERING_SOPE.build_with_noise(build_noise).geometry.scale.stalk_height;
			(WanderingAcaciaKind::Sope(SopesBanyan::grow_num(variant).0), world_size)
		}
		WanderingAcaciaCell::WanderingVaseTree => {
			let (tree, world_size) = WanderingVaseTree::grow_num(variant);
			(WanderingAcaciaKind::Vase(tree), world_size)
		}
		WanderingAcaciaCell::WanderingPenmarchTorch => {
			let (tree, world_size) = WanderingPenmarchTorch::grow_num(variant);
			(WanderingAcaciaKind::Penmarch(tree), world_size)
		}
		WanderingAcaciaCell::WanderingKamakuraTorch => {
			let (tree, world_size) = WanderingKamakuraTorch::grow_num(variant);
			(WanderingAcaciaKind::Kamakura(tree), world_size)
		}
	};

	WanderingAcaciaPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(WanderingAcacia, WOODY_LOD);

#[cfg(test)]
mod tests;
