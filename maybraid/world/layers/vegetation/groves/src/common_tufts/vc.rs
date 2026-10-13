use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::QuantizedPlant;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use super::{
	definition, CommonTuftsCell, DRY_SCRUB, DRY_SCRUB_PATCH, SHORT_GREEN, SHORT_GREEN_PATCH,
	TALL_WILD, TALL_WILD_PATCH,
};
use crate::grove::vc_tuft::{
	grow_placed_tuft_params, tuft_grove_stick_nodes, TuftGroveBody, TuftGrovePlant,
	TuftGroveProxyHeights,
};
use crate::grove::{
	remixed_blade_tuft_plant, remixed_tuft_plant, FlatTerrainSample, GrovePreviewParams,
};

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.06, 1)
}

remixed_blade_tuft_plant!(ShortGreen, SHORT_GREEN, default_foliage());
remixed_blade_tuft_plant!(DryScrub, DRY_SCRUB, default_foliage());
remixed_blade_tuft_plant!(TallWild, TALL_WILD, default_foliage());
remixed_tuft_plant!(ShortGreenPatch, SHORT_GREEN_PATCH, default_foliage());
remixed_tuft_plant!(DryScrubPatch, DRY_SCRUB_PATCH, default_foliage());
remixed_tuft_plant!(TallWildPatch, TALL_WILD_PATCH, default_foliage());

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct CommonTuftsParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<CommonTuftsCell>,

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

impl Default for CommonTuftsParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(CommonTuftsParams, CommonTuftsCell);

impl CommonTuftsParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> CommonTufts {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> CommonTufts {
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
					CommonTuftsCell::ShortGreen => ShortGreen::grow_num(variant),
					CommonTuftsCell::DryScrub => DryScrub::grow_num(variant),
					CommonTuftsCell::TallWild => TallWild::grow_num(variant),
					CommonTuftsCell::ShortGreenPatch => ShortGreenPatch::grow_num(variant),
					CommonTuftsCell::DryScrubPatch => DryScrubPatch::grow_num(variant),
					CommonTuftsCell::TallWildPatch => TallWildPatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		);
		CommonTufts {
			body: TuftGroveBody::from_plants(
				plants,
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::SHORT,
			),
		}
	}
}

#[derive(Clone, Debug, Component)]
pub struct CommonTufts {
	body: TuftGroveBody,
}

impl CommonTufts {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		self.body.plants()
	}
}

impl VegetationComponents for CommonTufts {
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

crate::impl_tuft_grove_lod!(CommonTufts);
