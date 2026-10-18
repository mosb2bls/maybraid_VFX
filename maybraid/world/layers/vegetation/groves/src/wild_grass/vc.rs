use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::QuantizedPlant;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use super::{
	definition, WildGrassCell, BLOOMING_GRASS, BLOOMING_GRASS_PATCH, BLUE_TROPICAL,
	BLUE_TROPICAL_PATCH, GOLDEN_GRASS, GOLDEN_GRASS_PATCH, MEADOW_GREEN, MEADOW_GREEN_PATCH,
	PALE_FIELD, PALE_FIELD_PATCH, RED_PRAIRIE, RED_PRAIRIE_PATCH,
};
use crate::grove::vc_tuft::{
	grow_placed_tuft_params, tuft_grove_stick_nodes, TuftGroveBody, TuftGrovePlant,
	TuftGroveProxyHeights,
};
use crate::grove::{
	remixed_blade_tuft_plant, remixed_tuft_plant, FlatTerrainSample, GrovePreviewParams,
};

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.12, 1)
}

remixed_blade_tuft_plant!(MeadowGreen, MEADOW_GREEN, default_foliage());
remixed_blade_tuft_plant!(GoldenGrass, GOLDEN_GRASS, default_foliage());
remixed_blade_tuft_plant!(RedPrairie, RED_PRAIRIE, default_foliage());
remixed_blade_tuft_plant!(BlueTropical, BLUE_TROPICAL, default_foliage());
remixed_blade_tuft_plant!(PaleField, PALE_FIELD, default_foliage());
remixed_blade_tuft_plant!(BloomingGrass, BLOOMING_GRASS, default_foliage());
remixed_tuft_plant!(MeadowGreenPatch, MEADOW_GREEN_PATCH, default_foliage());
remixed_tuft_plant!(GoldenGrassPatch, GOLDEN_GRASS_PATCH, default_foliage());
remixed_tuft_plant!(RedPrairiePatch, RED_PRAIRIE_PATCH, default_foliage());
remixed_tuft_plant!(BlueTropicalPatch, BLUE_TROPICAL_PATCH, default_foliage());
remixed_tuft_plant!(PaleFieldPatch, PALE_FIELD_PATCH, default_foliage());
remixed_tuft_plant!(BloomingGrassPatch, BLOOMING_GRASS_PATCH, default_foliage());

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct WildGrassParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<WildGrassCell>,

	#[arg(
		long,
		default_value = "0,1,0.12,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Foliage Surface Noise",
	)]
	pub foliage_noise: NoiseParams,

	#[arg(long, default_value_t = 0)]
	pub merge_collections: usize,

	#[arg(long, default_value_t = 100)]
	pub patch_variants: u32,
}

impl Default for WildGrassParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.12, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(WildGrassParams, WildGrassCell);

impl WildGrassParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> WildGrass {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> WildGrass {
		let foliage_noise = self.foliage_noise;
		let plants = grow_placed_tuft_params(
			&self.placements_on(world),
			foliage_noise,
			self.merge_collections,
			self.patch_variants,
			&self.extent,
			|cell, variant| {
				let mix = cell.palette_mix();
				let (patch, world_size) = match cell {
					WildGrassCell::MeadowGreen => MeadowGreen::grow_num(variant),
					WildGrassCell::GoldenGrass => GoldenGrass::grow_num(variant),
					WildGrassCell::RedPrairie => RedPrairie::grow_num(variant),
					WildGrassCell::BlueTropical => BlueTropical::grow_num(variant),
					WildGrassCell::PaleField => PaleField::grow_num(variant),
					WildGrassCell::BloomingGrass => BloomingGrass::grow_num(variant),
					WildGrassCell::MeadowGreenPatch => MeadowGreenPatch::grow_num(variant),
					WildGrassCell::GoldenGrassPatch => GoldenGrassPatch::grow_num(variant),
					WildGrassCell::RedPrairiePatch => RedPrairiePatch::grow_num(variant),
					WildGrassCell::BlueTropicalPatch => BlueTropicalPatch::grow_num(variant),
					WildGrassCell::PaleFieldPatch => PaleFieldPatch::grow_num(variant),
					WildGrassCell::BloomingGrassPatch => BloomingGrassPatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		);
		WildGrass {
			body: TuftGroveBody::from_plants(
				plants,
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::MID,
			),
		}
	}
}

#[derive(Clone, Debug, Component)]
pub struct WildGrass {
	body: TuftGroveBody,
}

impl WildGrass {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		self.body.plants()
	}
}

impl VegetationComponents for WildGrass {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		tuft_grove_stick_nodes(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.body.foliage_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.body.structural_lod())
	}
}

crate::impl_tuft_grove_lod!(WildGrass);
