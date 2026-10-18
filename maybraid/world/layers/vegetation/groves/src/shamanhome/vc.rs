use super::WOODY_LOD;
use std::sync::Arc;

use super::variants::shamanhome_banyan::SopeBanyanSamples;

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
use sbs_trees::{BraidOakTree, DatePalm, DatePalmParams, QuantizedPlant, SopesBanyan};
use vegetation_components::{FoliageNode, Placement, VegetationComponents};

use super::{definition, ShamanhomeCell, ShamanhomeItem, RITUAL_DATE_PALM};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_crown, canopy_proxy_site,
	foliage_low_canopy_balls, frond_material_from_palette, grove_structural_footprint,
	lazy_flattened_plant_chunks, nest_flattened_plant_chunk, placed_palm_low_fronds,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShamanhomeParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<ShamanhomeCell>,
}

impl Default for ShamanhomeParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.25, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(ShamanhomeParams, ShamanhomeCell);

impl ShamanhomeParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> Shamanhome {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> Shamanhome {
		Shamanhome::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(RitualDatePalm, DatePalm, DatePalmParams, RITUAL_DATE_PALM);

#[derive(Clone)]
enum ShamanhomeKind {
	Oak(Arc<BraidOakTree>),
	Date(Arc<DatePalm>),
	Sope(Arc<SopesBanyan>),
}

#[derive(Clone)]
pub struct ShamanhomePlant {
	pub placement: Placement,
	kind: ShamanhomeKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct Shamanhome {
	pub plants: Arc<[ShamanhomePlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl Shamanhome {
	pub fn from_placements(
		placements: &[GroveCellVariant<ShamanhomeCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[ShamanhomePlant]> = placements
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
				ShamanhomeKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ShamanhomeKind::Date(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				ShamanhomeKind::Sope(t) => nest_flattened_plant_chunk(
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
					ShamanhomeKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
					ShamanhomeKind::Date(t) => canopy_proxy_crown(t, plant.placement, material),
					ShamanhomeKind::Sope(t) => canopy_proxy_site(t, plant.placement, material),
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
				ShamanhomeKind::Oak(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				ShamanhomeKind::Date(t) => {
					nodes.extend(placed_palm_low_fronds(
						t.as_ref(),
						plant.placement,
						&plant.stick_material,
						material,
						&plant.frond_material,
					));
				}
				ShamanhomeKind::Sope(t) => {
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
	placed: &GroveCellVariant<ShamanhomeCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> ShamanhomePlant {
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
		ShamanhomeItem::BraidOak(oak) => {
			let world_size = oak.build_with_noise(build_noise).height();
			ShamanhomePlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: ShamanhomeKind::Oak(BraidOakTree::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		ShamanhomeItem::DatePalm(_) => {
			let (tree, world_size) = RitualDatePalm::grow_num(variant);
			ShamanhomePlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: ShamanhomeKind::Date(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		ShamanhomeItem::SopeBanyan(banyan) => {
			let world_size =
				BuildWithNoise::<SopeBanyanSamples>::build_with_noise(banyan, build_noise)
					.geometry
					.scale
					.stalk_height;
			ShamanhomePlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: ShamanhomeKind::Sope(SopesBanyan::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(Shamanhome, WOODY_LOD, low_nodes);

#[cfg(test)]
mod tests;
