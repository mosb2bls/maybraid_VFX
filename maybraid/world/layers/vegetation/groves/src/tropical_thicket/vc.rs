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
use sbs_trees::{HighBushShoots, HonuBanyan, PalmBush, PalmBushParams, QuantizedPlant};
use vegetation_components::{FoliageNode, Placement, VegetationComponents};

use super::{
	definition, TropicalThicketCell, TropicalThicketPalm, BROAD_WET_PALM_BUSH, FLOWERING_HIGH_BUSH,
	LARGE_PALM_BUSH, MINI_HONU_BANYAN, MODERATE_HIGH_BUSH, RED_STEM_PALM_BUSH,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_crown, canopy_proxy_site,
	foliage_low_canopy_balls, frond_material_from_palette, grove_structural_footprint,
	lazy_flattened_plant_chunks, nest_flattened_plant_chunk, placed_palm_low_fronds,
	placement_noise, remixed_bush_plant, stick_material_from_palette, unit_build_noise,
	CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

/// Authoring / CLI parameters for Tropical Thicket.
#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalThicketParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TropicalThicketCell>,
}

impl Default for TropicalThicketParams {
	fn default() -> Self {
		Self { preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()) }
	}
}

crate::impl_grove_preview_params!(TropicalThicketParams, TropicalThicketCell);

impl TropicalThicketParams {
	// preview accessors via impl_grove_preview_params!
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<TropicalThicketCell>>,
		terrain: FlatTerrainSample,
	) -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(terrain)
				.with_resolved_placements(resolved_placements),
		}
	}

	pub fn build(&self) -> TropicalThicket {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalThicket {
		TropicalThicket::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

fn thicket_palm_unit(authored: &TropicalThicketPalm, num: u32) -> (PalmBush, f32) {
	let mut geometry = authored.build_with_noise(unit_build_noise(num));
	let detail = PalmBushParams::unit_detail_from_num(num);
	geometry.crown.ring_count = detail.geometry.crown.ring_count;
	geometry.crown.fronds_per_ring = detail.geometry.crown.fronds_per_ring;
	let (unit, world_size) = PalmBushParams::new(geometry).into_unit_from_num(num);
	(unit.build(), world_size)
}

struct LargePalmBush;
struct BroadWetPalmBush;
struct RedStemPalmBush;

impl QuantizedPlant for LargePalmBush {
	type Unit = PalmBush;
	fn build_unit(num: u32) -> (PalmBush, f32) {
		thicket_palm_unit(&LARGE_PALM_BUSH, num)
	}
}

impl QuantizedPlant for BroadWetPalmBush {
	type Unit = PalmBush;
	fn build_unit(num: u32) -> (PalmBush, f32) {
		thicket_palm_unit(&BROAD_WET_PALM_BUSH, num)
	}
}

impl QuantizedPlant for RedStemPalmBush {
	type Unit = PalmBush;
	fn build_unit(num: u32) -> (PalmBush, f32) {
		thicket_palm_unit(&RED_STEM_PALM_BUSH, num)
	}
}

remixed_bush_plant!(ModerateHighBush, MODERATE_HIGH_BUSH);
remixed_bush_plant!(FloweringHighBush, FLOWERING_HIGH_BUSH);

#[derive(Clone)]
enum TropicalThicketKind {
	/// Ground palm bush; crown counts keyed by [`PalmBushParams::unit_detail_from_num`].
	Palm(Arc<PalmBush>),
	Banyan(Arc<HonuBanyan>),
	Bush(Arc<HighBushShoots>),
}

#[derive(Clone)]
pub struct TropicalThicketPlant {
	pub placement: Placement,
	kind: TropicalThicketKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TropicalThicket {
	pub plants: Arc<[TropicalThicketPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl TropicalThicket {
	pub fn from_placements(
		placements: &[GroveCellVariant<TropicalThicketCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[TropicalThicketPlant]> = placements
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
				TropicalThicketKind::Palm(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalThicketKind::Banyan(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				TropicalThicketKind::Bush(t) => nest_flattened_plant_chunk(
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
					TropicalThicketKind::Palm(t) => {
						canopy_proxy_crown(t, plant.placement, material)
					}
					TropicalThicketKind::Banyan(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
					TropicalThicketKind::Bush(t) => canopy_proxy_site(t, plant.placement, material),
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
				TropicalThicketKind::Palm(t) => {
					nodes.extend(placed_palm_low_fronds(
						t.as_ref(),
						plant.placement,
						&plant.stick_material,
						material,
						&plant.frond_material,
					));
				}
				TropicalThicketKind::Banyan(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				TropicalThicketKind::Bush(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
			}
		}
		nodes.extend(foliage_low_canopy_balls(sites));
		nodes
	}
}

fn grow_plant(
	placed: &GroveCellVariant<TropicalThicketCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> TropicalThicketPlant {
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
		TropicalThicketCell::LargePalmBush => {
			let (tree, world_size) = LargePalmBush::grow_num(variant);
			(TropicalThicketKind::Palm(tree), world_size)
		}
		TropicalThicketCell::BroadWetPalmBush => {
			let (tree, world_size) = BroadWetPalmBush::grow_num(variant);
			(TropicalThicketKind::Palm(tree), world_size)
		}
		TropicalThicketCell::RedStemPalmBush => {
			let (tree, world_size) = RedStemPalmBush::grow_num(variant);
			(TropicalThicketKind::Palm(tree), world_size)
		}
		TropicalThicketCell::ModerateHighBush => {
			let (tree, world_size) = ModerateHighBush::grow_num(variant);
			(TropicalThicketKind::Bush(tree), world_size)
		}
		TropicalThicketCell::FloweringHighBush => {
			let (tree, world_size) = FloweringHighBush::grow_num(variant);
			(TropicalThicketKind::Bush(tree), world_size)
		}
		TropicalThicketCell::MiniHonuBanyan => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size =
				MINI_HONU_BANYAN.build_with_noise(build_noise).geometry.scale.tree_height;
			(TropicalThicketKind::Banyan(HonuBanyan::grow_num(variant).0), world_size)
		}
	};

	TropicalThicketPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(TropicalThicket, WOODY_LOD, low_nodes);

#[cfg(test)]
mod tests;
