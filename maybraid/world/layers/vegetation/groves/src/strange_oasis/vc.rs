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
use sbs_geometry::DatePalmSbs;
use sbs_trees::{
	DatePalm, DatePalmParams, PalmCrown, PalmCrownParams, PenmarchTorch, PenmarchTorchParams,
	QuantizedPlant, StorybookTree, StorybookTreeParams,
};
use vegetation_components::{
	FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
};

use super::{
	definition, StrangeOasisCell, COMPACT_DATE_PALM, OASIS_STORYBOOK, RED_TORCH_ACCENT,
	TORCH_ACCENT,
};
use crate::grove::vc_tuft::patch_variant_index;
use crate::grove::{
	canopy_ball_material_from_palette, canopy_proxy_crown, canopy_proxy_site,
	foliage_low_canopy_balls, frond_material_from_palette, grove_structural_footprint,
	lazy_flattened_plant_chunks, nest_flattened_plant_chunk, placed_palm_low_fronds,
	placement_noise, remixed_sbs_plant, stick_material_from_palette, unit_build_noise,
	CanopyProxySite, FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

/// Authoring / CLI parameters for Strange Oasis.
#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct StrangeOasisParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<StrangeOasisCell>,
}

impl Default for StrangeOasisParams {
	fn default() -> Self {
		Self { preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()) }
	}
}

crate::impl_grove_preview_params!(StrangeOasisParams, StrangeOasisCell);

impl StrangeOasisParams {
	// preview accessors via impl_grove_preview_params!
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<StrangeOasisCell>>,
		terrain: FlatTerrainSample,
	) -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(terrain)
				.with_resolved_placements(resolved_placements),
		}
	}

	pub fn build(&self) -> StrangeOasis {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> StrangeOasis {
		StrangeOasis::from_placements(
			&self.placements_on(world),
			self.grove.noise,
			&self.extent,
			self.tree_variants,
		)
	}
}

/// Oasis date palm: trunk sticks + unit [`PalmCrown`] foliage (no DatePalm fronds).
#[derive(Clone, Component)]
pub struct OasisDatePalm {
	pub trunk: DatePalm,
	pub crown: PalmCrown,
	pub crown_local: Placement,
}

impl VegetationComponents for OasisDatePalm {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		self.trunk.stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes = self
			.crown
			.foliage_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(|mut node| {
				node.placement = self.crown_local.compose_child(node.placement);
				node
			})
			.collect::<Vec<_>>();
		Layers::from_free(nodes)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		let lod = self.crown.structural_lod()?;
		let scale = self.crown_local.scale.abs().max_element().max(1e-4);
		let center = self.crown_local.compose_child(Placement::new(lod.center, 0.0)).translation;
		Some(
			StructuralLod::new(center, (lod.tree_radius * scale).max(1e-4))
				.with_factors(lod.high_factor, lod.medium_factor, lod.low_factor)
				.with_preserve_ultra_low(lod.preserve_ultra_low),
		)
	}
}

struct CompactDatePalm;

impl QuantizedPlant for CompactDatePalm {
	type Unit = OasisDatePalm;

	fn build_unit(num: u32) -> (OasisDatePalm, f32) {
		let geometry = COMPACT_DATE_PALM.build_with_noise(unit_build_noise(num));
		let mut trunk_params = DatePalmParams::default();
		trunk_params.geometry = geometry;
		let (unit_trunk, trunk_world) = trunk_params.into_unit_from_num(num);
		let trunk = unit_trunk.build();
		let tip = DatePalmSbs::trunk_tip_from_chain(&trunk.chain);
		let (unit_crown, crown_size) = PalmCrownParams::unit_full_for_height_from_num(1.0, num);
		let crown = unit_crown.build();
		let crown_local = Placement::new(tip, 0.0).with_scale(Vec3::splat(crown_size.max(1e-4)));
		(OasisDatePalm { trunk, crown, crown_local }, trunk_world)
	}
}

remixed_sbs_plant!(TorchAccent, PenmarchTorch, PenmarchTorchParams, TORCH_ACCENT);
remixed_sbs_plant!(RedTorchAccent, PenmarchTorch, PenmarchTorchParams, RED_TORCH_ACCENT);
remixed_sbs_plant!(OasisStorybook, StorybookTree, StorybookTreeParams, OASIS_STORYBOOK);

#[derive(Clone)]
enum StrangeOasisKind {
	/// Columnar trunk + unit PalmCrown at tip
	/// ([`PalmCrownParams::unit_full_for_height_from_num`]).
	DatePalm(Arc<OasisDatePalm>),
	Torch(Arc<PenmarchTorch>),
	Storybook(Arc<StorybookTree>),
}

#[derive(Clone)]
pub struct StrangeOasisPlant {
	pub placement: Placement,
	kind: StrangeOasisKind,
	stick_material: MaterialRef,
	ball_material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct StrangeOasis {
	pub plants: Arc<[StrangeOasisPlant]>,
	pub structural_center: Vec3,
	pub footprint_radius: f32,
	pub extent: GroveExtent,
}

impl StrangeOasis {
	pub fn from_placements(
		placements: &[GroveCellVariant<StrangeOasisCell>],
		grove_noise: NoiseParams,
		extent: &GroveExtent,
		tree_variants: u32,
	) -> Self {
		let plants: Arc<[StrangeOasisPlant]> = placements
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
				StrangeOasisKind::DatePalm(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				StrangeOasisKind::Torch(t) => nest_flattened_plant_chunk(
					Arc::clone(t),
					plant.placement,
					&plant.stick_material,
					&plant.ball_material,
					&plant.frond_material,
					&plant_lod,
					level,
				),
				StrangeOasisKind::Storybook(t) => nest_flattened_plant_chunk(
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
					StrangeOasisKind::DatePalm(t) => {
						canopy_proxy_crown(t, plant.placement, material)
					}
					StrangeOasisKind::Torch(t) => canopy_proxy_site(t, plant.placement, material),
					StrangeOasisKind::Storybook(t) => {
						canopy_proxy_site(t, plant.placement, material)
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
				StrangeOasisKind::DatePalm(t) => {
					nodes.extend(placed_palm_low_fronds(
						t.as_ref(),
						plant.placement,
						&plant.stick_material,
						material,
						&plant.frond_material,
					));
				}
				StrangeOasisKind::Torch(t) => {
					if let Some(site) = canopy_proxy_site(t, plant.placement, material) {
						sites.push(site);
					}
				}
				StrangeOasisKind::Storybook(t) => {
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
	placed: &GroveCellVariant<StrangeOasisCell>,
	grove_noise: NoiseParams,
	tree_variants: u32,
) -> StrangeOasisPlant {
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
		StrangeOasisCell::CompactDatePalm => {
			let (tree, world_size) = CompactDatePalm::grow_num(variant);
			(StrangeOasisKind::DatePalm(tree), world_size)
		}
		StrangeOasisCell::TorchAccent => {
			let (tree, world_size) = TorchAccent::grow_num(variant);
			(StrangeOasisKind::Torch(tree), world_size)
		}
		StrangeOasisCell::RedTorchAccent => {
			let (tree, world_size) = RedTorchAccent::grow_num(variant);
			(StrangeOasisKind::Torch(tree), world_size)
		}
		StrangeOasisCell::OasisStorybook => {
			let (tree, world_size) = OasisStorybook::grow_num(variant);
			(StrangeOasisKind::Storybook(tree), world_size)
		}
	};

	StrangeOasisPlant {
		placement: Placement::new(placed.position, 0.0)
			.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
		kind,
		stick_material,
		ball_material,
		frond_material,
	}
}

crate::impl_woody_grove_lod!(StrangeOasis, WOODY_LOD, low_nodes);

#[cfg(test)]
mod tests;
