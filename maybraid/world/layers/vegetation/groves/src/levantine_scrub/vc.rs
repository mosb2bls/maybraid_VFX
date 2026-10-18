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
	BraidOakTree, HighBushShoots, PenmarchTorch, PenmarchTorchParams, QuantizedPlant,
	RorysHeadTrained, RorysHeadTrainedParams, SimplemansHedge, SimplemansHedgeParams, VaseTree,
	VaseTreeParams,
};
use vegetation_components::{Placement, StickNode, VegetationComponents};

use super::{
	definition, LevantineScrubCell, LevantineScrubHedge, DRY_HIGH_BUSH, DRY_RORY_HEAD,
	RED_OLIVE_TORCH, SCRUB_HEDGE, SMALL_BRAID_OAK, SMALL_PENMARCH_TORCH, SMALL_VASE_TREE,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site,
	frond_material_from_palette, grove_structural_footprint, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, placement_noise, remixed_bush_plant, remixed_sbs_plant,
	stick_material_from_palette, unit_build_noise, CanopyProxySite, FlatTerrainSample,
	GroveCellVariant, GroveExtent, GrovePreviewParams,
};

/// Authoring / CLI parameters for Levantine Scrub.
#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct LevantineScrubParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<LevantineScrubCell>,
}

impl Default for LevantineScrubParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.25, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(LevantineScrubParams, LevantineScrubCell);

impl LevantineScrubParams {
	// preview accessors via impl_grove_preview_params!
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<LevantineScrubCell>>,
		terrain: FlatTerrainSample,
	) -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(terrain)
				.with_resolved_placements(resolved_placements),
		}
	}

	pub fn build(&self) -> LevantineScrub {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> LevantineScrub {
		LevantineScrub::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(DryRoryHead, RorysHeadTrained, RorysHeadTrainedParams, DRY_RORY_HEAD);
remixed_sbs_plant!(SmallVaseTree, VaseTree, VaseTreeParams, SMALL_VASE_TREE);
remixed_bush_plant!(DryHighBush, DRY_HIGH_BUSH);
remixed_sbs_plant!(SmallPenmarchTorch, PenmarchTorch, PenmarchTorchParams, SMALL_PENMARCH_TORCH);
remixed_sbs_plant!(RedOliveTorch, PenmarchTorch, PenmarchTorchParams, RED_OLIVE_TORCH);

fn scrub_hedge_unit(authored: &LevantineScrubHedge, num: u32) -> (SimplemansHedge, f32) {
	let samples = authored.build_with_noise(unit_build_noise(num));
	let (unit, world_size) = SimplemansHedgeParams::new(
		samples.height,
		samples.footprint_xz,
		samples.density,
		samples.seed,
	)
	.into_unit_from_num(num);
	(unit.build(), world_size)
}

struct ScrubHedge;

impl QuantizedPlant for ScrubHedge {
	type Unit = SimplemansHedge;
	fn build_unit(num: u32) -> (SimplemansHedge, f32) {
		scrub_hedge_unit(&SCRUB_HEDGE, num)
	}
}

#[derive(Clone)]
enum LevantineScrubKind {
	Rory(Arc<RorysHeadTrained>),
	Vase(Arc<VaseTree>),
	Bush(Arc<HighBushShoots>),
	Torch(Arc<PenmarchTorch>),
	Oak(Arc<BraidOakTree>),
	Hedge(Arc<SimplemansHedge>),
}

/// One scrub plant with placement and palette materials.
#[derive(Clone)]
pub struct LevantineScrubPlant {
	pub placement: Placement,
	kind: LevantineScrubKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

/// Built Levantine Scrub grove (`LodScene` nests plant `ComponentsOnly` hosts).
#[derive(Clone, Component)]
pub struct LevantineScrub {
	pub plants: Arc<[LevantineScrubPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl LevantineScrub {
	pub fn from_placements(
		placements: &[GroveCellVariant<LevantineScrubCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[LevantineScrubPlant]> = placements
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
				LevantineScrubKind::Rory(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LevantineScrubKind::Vase(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LevantineScrubKind::Bush(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LevantineScrubKind::Torch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LevantineScrubKind::Oak(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				LevantineScrubKind::Hedge(t) => nest_flattened_plant_chunk(
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
					LevantineScrubKind::Rory(t) => vec![
						canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
							.crown,
					],
					LevantineScrubKind::Vase(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					LevantineScrubKind::Bush(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					LevantineScrubKind::Torch(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					LevantineScrubKind::Oak(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					LevantineScrubKind::Hedge(t) => {
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
				LevantineScrubKind::Rory(t) => {
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
	placed: &GroveCellVariant<LevantineScrubCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> LevantineScrubPlant {
	let variant = patch_variant_index(placed.position, tree_variants);
	let palette_noise = placement_noise(grove_noise, placed.position);
	let stick_seed = palette_noise.seed;
	let canopy_seed = palette_noise.seed.wrapping_add(31);
	let stick_material =
		stick_material_from_palette(placed.variant.stick_palette_mix(), stick_seed);
	let canopy_palette =
		placed.variant.canopy_palette_mix().or_else(|| placed.variant.palette_mix());
	let ball_material = canopy_ball_material_from_palette(canopy_palette, canopy_seed);
	let frond_material = frond_material_from_palette(canopy_palette, canopy_seed);

	let (kind, world_size) = match placed.variant {
		LevantineScrubCell::DryRoryHeadTrained => {
			let (tree, world_size) = DryRoryHead::grow_num(variant);
			(LevantineScrubKind::Rory(tree), world_size)
		}
		LevantineScrubCell::SmallVaseTree => {
			let (tree, world_size) = SmallVaseTree::grow_num(variant);
			(LevantineScrubKind::Vase(tree), world_size)
		}
		LevantineScrubCell::DryHighBush => {
			let (tree, world_size) = DryHighBush::grow_num(variant);
			(LevantineScrubKind::Bush(tree), world_size)
		}
		LevantineScrubCell::SmallPenmarchTorch => {
			let (tree, world_size) = SmallPenmarchTorch::grow_num(variant);
			(LevantineScrubKind::Torch(tree), world_size)
		}
		LevantineScrubCell::RedOliveTorch => {
			let (tree, world_size) = RedOliveTorch::grow_num(variant);
			(LevantineScrubKind::Torch(tree), world_size)
		}
		LevantineScrubCell::SmallBraidOak => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size = SMALL_BRAID_OAK.build_with_noise(build_noise).height();
			(LevantineScrubKind::Oak(BraidOakTree::grow_num(variant).0), world_size)
		}
		LevantineScrubCell::ScrubHedge => {
			let (tree, world_size) = ScrubHedge::grow_num(variant);
			(LevantineScrubKind::Hedge(tree), world_size)
		}
	};

	LevantineScrubPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(LevantineScrub, WOODY_LOD, trunks);

#[cfg(test)]
mod tests;
