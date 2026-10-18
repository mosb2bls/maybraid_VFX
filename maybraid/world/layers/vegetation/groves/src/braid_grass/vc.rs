use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::QuantizedPlant;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use super::{
	definition, BraidGrassCell, DEEP_GREEN_BLADE, DEEP_GREEN_PATCH, FOUNTAIN_SPEAR, GREEN_SPEAR,
	JUNGLE_BLADE, JUNGLE_PATCH, PALE_REED_BLADE, PALE_REED_PATCH, RED_EDGE_BLADE,
};
use crate::grove::vc_tuft::{
	grow_placed_tuft_params, tuft_grove_stick_nodes, TuftGroveBody, TuftGrovePlant,
	TuftGroveProxyHeights,
};
use crate::grove::{
	remixed_blade_tuft_plant, remixed_spear_tuft_plant, remixed_tuft_plant, FlatTerrainSample,
	GrovePreviewParams,
};

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.06, 1)
}

remixed_blade_tuft_plant!(DeepGreenBlade, DEEP_GREEN_BLADE, default_foliage());
remixed_blade_tuft_plant!(PaleReedBlade, PALE_REED_BLADE, default_foliage());
remixed_blade_tuft_plant!(JungleBlade, JUNGLE_BLADE, default_foliage());
remixed_blade_tuft_plant!(RedEdgeBlade, RED_EDGE_BLADE, default_foliage());
remixed_spear_tuft_plant!(GreenSpear, GREEN_SPEAR, default_foliage());
remixed_spear_tuft_plant!(FountainSpear, FOUNTAIN_SPEAR, default_foliage());
remixed_tuft_plant!(DeepGreenPatch, DEEP_GREEN_PATCH, default_foliage());
remixed_tuft_plant!(PaleReedPatch, PALE_REED_PATCH, default_foliage());
remixed_tuft_plant!(JunglePatch, JUNGLE_PATCH, default_foliage());

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct BraidGrassParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<BraidGrassCell>,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
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

impl Default for BraidGrassParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(BraidGrassParams, BraidGrassCell);

impl BraidGrassParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> BraidGrass {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> BraidGrass {
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
					BraidGrassCell::DeepGreenBlade => DeepGreenBlade::grow_num(variant),
					BraidGrassCell::PaleReedBlade => PaleReedBlade::grow_num(variant),
					BraidGrassCell::JungleBlade => JungleBlade::grow_num(variant),
					BraidGrassCell::RedEdgeBlade => RedEdgeBlade::grow_num(variant),
					BraidGrassCell::GreenSpear => GreenSpear::grow_num(variant),
					BraidGrassCell::FountainSpear => FountainSpear::grow_num(variant),
					BraidGrassCell::DeepGreenPatch => DeepGreenPatch::grow_num(variant),
					BraidGrassCell::PaleReedPatch => PaleReedPatch::grow_num(variant),
					BraidGrassCell::JunglePatch => JunglePatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		);
		BraidGrass {
			body: TuftGroveBody::from_plants(
				plants,
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::TALL,
			),
		}
	}
}

#[derive(Clone, Debug, Component)]
pub struct BraidGrass {
	body: TuftGroveBody,
}

impl BraidGrass {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		self.body.plants()
	}
}

impl VegetationComponents for BraidGrass {
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

crate::impl_tuft_grove_lod!(BraidGrass);
