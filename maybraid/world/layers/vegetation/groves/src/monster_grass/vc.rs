use super::{
	MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR, MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
	MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
};
use std::sync::Arc;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::QuantizedPlant;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use super::{
	definition, MonsterGrassCell, BROAD_JUNGLE_BLADE, BROAD_JUNGLE_BLADE_PATCH, GIANT_WET_BLADE,
	GIANT_WET_BLADE_PATCH, PALE_GIANT_REED, PALE_GIANT_REED_PATCH, RED_RIBBED_BLADE,
	RED_RIBBED_BLADE_PATCH,
};
use crate::grove::{
	remixed_tuft_plant,
	vc_tuft::{grow_placed_tuft_params, TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights},
	FlatTerrainSample, GroveCellVariant, GroveExtent, GrovePreviewParams,
};

/// Authoring / CLI parameters for Monster Grass.
#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct MonsterGrassParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<MonsterGrassCell>,

	#[arg(
		long,
		default_value = "0,1,0.20,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	/// Cap foliage LOD collections after growing placements into square XZ bins
	/// (`ceil(sqrt(n))` on a side; `0` = one collection per placement).
	#[arg(long, default_value_t = 0)]
	pub merge_collections: usize,

	/// Number of unit-scale tuft-patch archetypes (`unit_from_num(0..n)`). Caps unique
	/// merged-mesh handles for High/Medium.
	#[arg(long, default_value_t = 100)]
	pub patch_variants: u32,
}

impl Default for MonsterGrassParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.20, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(MonsterGrassParams, MonsterGrassCell);

impl MonsterGrassParams {
	// preview accessors via impl_grove_preview_params!
	pub fn with_resolved_placements(
		resolved_placements: Vec<GroveCellVariant<MonsterGrassCell>>,
		terrain: FlatTerrainSample,
		foliage_noise: NoiseParams,
	) -> Self {
		Self {
			preview: GrovePreviewParams::default()
				.with_terrain(terrain)
				.with_resolved_placements(resolved_placements),
			foliage_noise,
			merge_collections: 0,
			patch_variants: 100,
		}
	}

	pub fn build(&self) -> MonsterGrass {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> MonsterGrass {
		MonsterGrass::from_placements(
			&self.placements_on(world),
			self.foliage_noise,
			&self.extent,
			self.merge_collections,
			self.patch_variants,
		)
	}
}

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.20, 1)
}

remixed_tuft_plant!(GiantWetBlade, GIANT_WET_BLADE, default_foliage());
remixed_tuft_plant!(BroadJungleBlade, BROAD_JUNGLE_BLADE, default_foliage());
remixed_tuft_plant!(PaleGiantReed, PALE_GIANT_REED, default_foliage());
remixed_tuft_plant!(RedRibbedBlade, RED_RIBBED_BLADE, default_foliage());
remixed_tuft_plant!(GiantWetBladePatch, GIANT_WET_BLADE_PATCH, default_foliage());
remixed_tuft_plant!(BroadJungleBladePatch, BROAD_JUNGLE_BLADE_PATCH, default_foliage());
remixed_tuft_plant!(PaleGiantReedPatch, PALE_GIANT_REED_PATCH, default_foliage());
remixed_tuft_plant!(RedRibbedBladePatch, RED_RIBBED_BLADE_PATCH, default_foliage());

/// One grove-local [`TuftPatch`] collection (placement already baked when merged).
pub type MonsterGrassPlant = TuftGrovePlant;

const PROXY_HEIGHT_LOW: f32 = 4.5;
/// Carpet float height along the surface normal; kept small — this is not blade length.
const PROXY_HEIGHT_ULTRA: f32 = 0.6;

/// Built Monster Grass grove: composed [`TuftPatch`] plants for VegetationComponents.
#[derive(Clone, Debug, Component)]
pub struct MonsterGrass {
	pub plants: Arc<[MonsterGrassPlant]>,
	body: TuftGroveBody,
}

impl MonsterGrass {
	/// Grow every placement into a unit [`TuftPatch`] archetype; fold when
	/// `merge_collections > 0`.
	pub fn from_placements(
		placements: &[GroveCellVariant<MonsterGrassCell>],
		foliage_noise: NoiseParams,
		extent: &GroveExtent,
		merge_collections: usize,
		patch_variants: u32,
	) -> Self {
		let tuft_plants = grow_placed_tuft_params(
			placements,
			foliage_noise,
			merge_collections,
			patch_variants,
			extent,
			|cell, variant| {
				let mix = cell.palette_mix();
				let (patch, world_size) = match cell {
					MonsterGrassCell::GiantWetBlade => GiantWetBlade::grow_num(variant),
					MonsterGrassCell::BroadJungleBlade => BroadJungleBlade::grow_num(variant),
					MonsterGrassCell::PaleGiantReed => PaleGiantReed::grow_num(variant),
					MonsterGrassCell::RedRibbedBlade => RedRibbedBlade::grow_num(variant),
					MonsterGrassCell::GiantWetBladePatch => GiantWetBladePatch::grow_num(variant),
					MonsterGrassCell::BroadJungleBladePatch => {
						BroadJungleBladePatch::grow_num(variant)
					}
					MonsterGrassCell::PaleGiantReedPatch => PaleGiantReedPatch::grow_num(variant),
					MonsterGrassCell::RedRibbedBladePatch => RedRibbedBladePatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		);
		let body = TuftGroveBody::from_plants(
			tuft_plants,
			extent,
			definition().cell_extent_xz,
			TuftGroveProxyHeights { low: PROXY_HEIGHT_LOW, ultra: PROXY_HEIGHT_ULTRA },
		);
		Self { plants: Arc::clone(&body.plants), body }
	}
}

impl VegetationComponents for MonsterGrass {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.body.foliage_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::new(self.body.structural_center, self.body.footprint_radius)
				.with_factors(
					MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR,
					MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
					MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
				)
				.with_preserve_ultra_low(true),
		)
	}
}

impl MonsterGrass {
	fn tuft_scene_chunks(
		&self,
		lod_ref: &lod::lod_ref::LodRef,
		level: lod::scene::LodSceneLevel,
	) -> lod::SceneChunk {
		match level {
			lod::scene::LodSceneLevel::High | lod::scene::LodSceneLevel::Medium => {
				self.body.high_medium_chunks(lod_ref, level)
			}
			_ => self.body.low_ultra_chunks(lod_ref, level),
		}
	}
}

crate::impl_tuft_grove_lod_emit!(MonsterGrass);
