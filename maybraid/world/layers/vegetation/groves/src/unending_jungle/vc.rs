use super::WOODY_LOD;
use std::sync::Arc;

use super::variants::unending_jungle_banyan::{HonuBanyanSamples, SopeBanyanSamples};

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
	HonuBanyan, JungleStorybookTree, PenmarchTorch, PenmarchTorchParams, QuantizedPlant,
	RorysHeadTrained, RorysHeadTrainedParams, SopesBanyan, StorybookTree, StorybookTreeParams,
	WaialeaPalm, WaialeaPalmParams,
};
use vegetation_components::{FoliageNode, Placement, StickNode, VegetationComponents};

use super::{
	definition, UnendingJungleCell, LOWER_STORYBOOK, PENUMARCH_ACCENT, RED_JUNGLE_TORCH,
	RORY_ACCENT, SMALL_HONU_BANYAN, SMALL_JUNGLE_STORYBOOK, SMALL_SOPE_BANYAN, WAIALEA_PALM_ACCENT,
};
use crate::grove::vc_tuft::{patch_variant_index, variant_noise};
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_rory, canopy_proxy_site, canopy_proxy_trunk,
	canopy_proxy_waialea, foliage_low_canopy_balls, frond_material_from_palette,
	grove_structural_footprint, lazy_flattened_plant_chunks, nest_flattened_plant_chunk,
	placed_palm_low_fronds, placement_noise, remixed_sbs_plant, stick_material_from_palette,
	CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct UnendingJungleParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<UnendingJungleCell>,
}

impl Default for UnendingJungleParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(FlatTerrainSample { elevation: 0.35, steepness: 0.15 }),
		}
	}
}

crate::impl_grove_preview_params!(UnendingJungleParams, UnendingJungleCell);

impl UnendingJungleParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> UnendingJungle {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> UnendingJungle {
		UnendingJungle::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

remixed_sbs_plant!(LowerStorybook, StorybookTree, StorybookTreeParams, LOWER_STORYBOOK);
remixed_sbs_plant!(PenmarchAccent, PenmarchTorch, PenmarchTorchParams, PENUMARCH_ACCENT);
remixed_sbs_plant!(RedJungleTorch, PenmarchTorch, PenmarchTorchParams, RED_JUNGLE_TORCH);
remixed_sbs_plant!(RoryAccent, RorysHeadTrained, RorysHeadTrainedParams, RORY_ACCENT);
remixed_sbs_plant!(WaialeaPalmAccent, WaialeaPalm, WaialeaPalmParams, WAIALEA_PALM_ACCENT);

#[derive(Clone)]
enum UnendingJungleKind {
	Honu(Arc<HonuBanyan>),
	Sope(Arc<SopesBanyan>),
	Storybook(Arc<StorybookTree>),
	JungleStorybook(Arc<JungleStorybookTree>),
	Torch(Arc<PenmarchTorch>),
	Rory(Arc<RorysHeadTrained>),
	Waialea(Arc<WaialeaPalm>),
}

#[derive(Clone)]
pub struct UnendingJunglePlant {
	pub placement: Placement,
	kind: UnendingJungleKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct UnendingJungle {
	pub plants: Arc<[UnendingJunglePlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl UnendingJungle {
	pub fn from_placements(
		placements: &[GroveCellVariant<UnendingJungleCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[UnendingJunglePlant]> = placements
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
				UnendingJungleKind::Honu(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::Sope(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::Storybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::JungleStorybook(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::Torch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::Rory(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				UnendingJungleKind::Waialea(t) => nest_flattened_plant_chunk(
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
					UnendingJungleKind::Honu(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					UnendingJungleKind::Sope(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					UnendingJungleKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					UnendingJungleKind::JungleStorybook(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					UnendingJungleKind::Torch(t) => {
						canopy_proxy_site(t, plant.placement, material).into_iter().collect()
					}
					UnendingJungleKind::Rory(t) => vec![
						canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
							.crown,
					],
					UnendingJungleKind::Waialea(t) => {
						canopy_proxy_waialea(t, plant.placement, &plant.stick_material, material)
					}
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
				UnendingJungleKind::Honu(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				UnendingJungleKind::Sope(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				UnendingJungleKind::Storybook(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				UnendingJungleKind::JungleStorybook(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				UnendingJungleKind::Torch(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				UnendingJungleKind::Rory(t) => {
					sites.push(
						canopy_proxy_rory(t, plant.placement, &plant.stick_material, material)
							.crown,
					);
				}
				UnendingJungleKind::Waialea(t) => {
					nodes.extend(placed_palm_low_fronds(
						t.as_ref(),
						plant.placement,
						&plant.stick_material,
						material,
						&plant.frond_material,
					));
					if let Some(trunk) =
						canopy_proxy_trunk(t, plant.placement, &plant.stick_material)
					{
						sites.push(trunk);
					}
				}
			}
		}
		nodes.extend(foliage_low_canopy_balls(sites));
		nodes
	}

	fn proxy_trunks(&self) -> Vec<StickNode> {
		self.plants
			.iter()
			.filter_map(|plant| match &plant.kind {
				UnendingJungleKind::Rory(t) => {
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
	placed: &GroveCellVariant<UnendingJungleCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> UnendingJunglePlant {
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
		UnendingJungleCell::SmallHonuBanyan => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size = BuildWithNoise::<HonuBanyanSamples>::build_with_noise(
				&SMALL_HONU_BANYAN,
				build_noise,
			)
			.geometry
			.scale
			.tree_height;
			(UnendingJungleKind::Honu(HonuBanyan::grow_num(variant).0), world_size)
		}
		UnendingJungleCell::SmallSopeBanyan => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size = BuildWithNoise::<SopeBanyanSamples>::build_with_noise(
				&SMALL_SOPE_BANYAN,
				build_noise,
			)
			.geometry
			.scale
			.stalk_height;
			(UnendingJungleKind::Sope(SopesBanyan::grow_num(variant).0), world_size)
		}
		UnendingJungleCell::LowerStorybook => {
			let (tree, world_size) = LowerStorybook::grow_num(variant);
			(UnendingJungleKind::Storybook(tree), world_size)
		}
		UnendingJungleCell::SmallJungleStorybook => {
			let build_noise = variant_noise(grove_noise, variant);
			let world_size = SMALL_JUNGLE_STORYBOOK.build_with_noise(build_noise).geometry.height();
			(
				UnendingJungleKind::JungleStorybook(JungleStorybookTree::grow_num(variant).0),
				world_size,
			)
		}
		UnendingJungleCell::PenmarchAccent => {
			let (tree, world_size) = PenmarchAccent::grow_num(variant);
			(UnendingJungleKind::Torch(tree), world_size)
		}
		UnendingJungleCell::RedJungleTorch => {
			let (tree, world_size) = RedJungleTorch::grow_num(variant);
			(UnendingJungleKind::Torch(tree), world_size)
		}
		UnendingJungleCell::RoryAccent => {
			let (tree, world_size) = RoryAccent::grow_num(variant);
			(UnendingJungleKind::Rory(tree), world_size)
		}
		UnendingJungleCell::WaialeaPalmAccent => {
			let (tree, world_size) = WaialeaPalmAccent::grow_num(variant);
			(UnendingJungleKind::Waialea(tree), world_size)
		}
	};

	UnendingJunglePlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(UnendingJungle, WOODY_LOD, trunks, low_nodes);

#[cfg(test)]
mod tests;
