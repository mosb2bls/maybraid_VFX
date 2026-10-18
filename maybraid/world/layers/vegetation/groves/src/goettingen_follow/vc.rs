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
use sbs_trees::{BraidOakTree, QuantizedPlant, StorybookTree, StorybookTreeParams};
use vegetation_components::{Placement, VegetationComponents};

use super::{definition, GoettingenFollowCell, GoettingenFollowItem, FOLLOW_STORYBOOK};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_site, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, CanopyProxySite,
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct GoettingenFollowParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<GoettingenFollowCell>,
}

impl Default for GoettingenFollowParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.25, steepness: 0.12 }),
		}
	}
}

crate::impl_grove_preview_params!(GoettingenFollowParams, GoettingenFollowCell);

impl GoettingenFollowParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> GoettingenFollow {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> GoettingenFollow {
		GoettingenFollow::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(FollowStorybook, StorybookTree, StorybookTreeParams, FOLLOW_STORYBOOK);

#[derive(Clone)]
enum GoettingenFollowKind {
	Oak(Arc<BraidOakTree>),
	Storybook(Arc<StorybookTree>),
}

#[derive(Clone)]
pub struct GoettingenFollowPlant {
	pub placement: Placement,
	kind: GoettingenFollowKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct GoettingenFollow {
	pub plants: Arc<[GoettingenFollowPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl GoettingenFollow {
	pub fn from_placements(
		placements: &[GroveCellVariant<GoettingenFollowCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[GoettingenFollowPlant]> = placements
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
				GoettingenFollowKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				GoettingenFollowKind::Storybook(t) => nest_flattened_plant_chunk(
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
					GoettingenFollowKind::Oak(t) => canopy_proxy_site(t, plant.placement, material),
					GoettingenFollowKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material)
					}
				}
			})
			.collect()
	}
}

fn grow_plant(
	placed: &GroveCellVariant<GoettingenFollowCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> GoettingenFollowPlant {
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
		GoettingenFollowItem::BraidOak(oak) => {
			let world_size = oak.build_with_noise(build_noise).height();
			GoettingenFollowPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: GoettingenFollowKind::Oak(BraidOakTree::grow_num(variant).0),
				stick_material,
				ball_material,
				frond_material,
			}
		}
		GoettingenFollowItem::Storybook(_) => {
			let (tree, world_size) = FollowStorybook::grow_num(variant);
			GoettingenFollowPlant {
				placement: Placement::new(placed.position, 0.0)
					.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
				kind: GoettingenFollowKind::Storybook(tree),
				stick_material,
				ball_material,
				frond_material,
			}
		}
	}
}

crate::impl_woody_grove_lod!(GoettingenFollow, WOODY_LOD);

#[cfg(test)]
mod tests;
