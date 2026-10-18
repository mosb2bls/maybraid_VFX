use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::QuantizedPlant;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use super::{
	definition, TallGrassCell, HAWAIIAN_RED, HAWAIIAN_RED_PATCH, PALE_REED, PALE_REED_PATCH,
	RIVER_GREEN, RIVER_GREEN_PATCH, TROPICAL_BLADE, TROPICAL_BLADE_PATCH,
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

remixed_blade_tuft_plant!(RiverGreen, RIVER_GREEN, default_foliage());
remixed_blade_tuft_plant!(PaleReed, PALE_REED, default_foliage());
remixed_blade_tuft_plant!(TropicalBlade, TROPICAL_BLADE, default_foliage());
remixed_blade_tuft_plant!(HawaiianRed, HAWAIIAN_RED, default_foliage());
remixed_tuft_plant!(RiverGreenPatch, RIVER_GREEN_PATCH, default_foliage());
remixed_tuft_plant!(PaleReedPatch, PALE_REED_PATCH, default_foliage());
remixed_tuft_plant!(TropicalBladePatch, TROPICAL_BLADE_PATCH, default_foliage());
remixed_tuft_plant!(HawaiianRedPatch, HAWAIIAN_RED_PATCH, default_foliage());

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TallGrassParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TallGrassCell>,

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

impl Default for TallGrassParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(TallGrassParams, TallGrassCell);

impl TallGrassParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TallGrass {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TallGrass {
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
					TallGrassCell::RiverGreen => RiverGreen::grow_num(variant),
					TallGrassCell::PaleReed => PaleReed::grow_num(variant),
					TallGrassCell::TropicalBlade => TropicalBlade::grow_num(variant),
					TallGrassCell::HawaiianRed => HawaiianRed::grow_num(variant),
					TallGrassCell::RiverGreenPatch => RiverGreenPatch::grow_num(variant),
					TallGrassCell::PaleReedPatch => PaleReedPatch::grow_num(variant),
					TallGrassCell::TropicalBladePatch => TropicalBladePatch::grow_num(variant),
					TallGrassCell::HawaiianRedPatch => HawaiianRedPatch::grow_num(variant),
				};
				(patch, world_size, mix)
			},
		);
		TallGrass {
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
pub struct TallGrass {
	body: TuftGroveBody,
}

impl TallGrass {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		self.body.plants()
	}
}

impl VegetationComponents for TallGrass {
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

crate::impl_tuft_grove_lod!(TallGrass);
