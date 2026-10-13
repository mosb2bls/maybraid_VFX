use std::sync::Arc;

use bevy::prelude::*;
use clap::Args;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use lod::SceneChunk;
use material_ref::MaterialRef;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};
use sbs_trees::{PalmBush, PalmBushParams, QuantizedPlant};
use vegetation_components::{
	FoliageNode, Layers, Placement, StickNode, StructuralLod, VegetationComponents,
};

use super::{
	definition, TropicalTuftsCell, BRIGHT_TUFT, BRIGHT_TUFT_PATCH, DEEP_TUFT, DEEP_TUFT_PATCH,
	JUVENILE_PALM_BUSH, SMALL_PALM_BUSH, YELLOW_GREEN_TUFT, YELLOW_GREEN_TUFT_PATCH,
};
use crate::grove::vc_tuft::{
	grow_tuft_plants, material_from_palette, patch_variant_index, tuft_grove_stick_nodes,
	unit_plant_from_grown, TuftGroveBody, TuftGrovePlant, TuftGroveProxyHeights,
};
use crate::grove::{
	flatten_foliage_nodes, frond_material_from_palette, placement_noise, remixed_blade_tuft_plant,
	remixed_sbs_plant, remixed_tuft_plant, FlatTerrainSample, GrovePreviewParams,
};

fn default_foliage() -> NoiseParams {
	NoiseParams::from_scalar(0.0, 1.0, 0.06, 1)
}

remixed_blade_tuft_plant!(BrightTuft, BRIGHT_TUFT, default_foliage());
remixed_blade_tuft_plant!(DeepTuft, DEEP_TUFT, default_foliage());
remixed_blade_tuft_plant!(YellowGreenTuft, YELLOW_GREEN_TUFT, default_foliage());
remixed_tuft_plant!(BrightTuftPatch, BRIGHT_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(DeepTuftPatch, DEEP_TUFT_PATCH, default_foliage());
remixed_tuft_plant!(YellowGreenTuftPatch, YELLOW_GREEN_TUFT_PATCH, default_foliage());
remixed_sbs_plant!(SmallPalmBush, PalmBush, PalmBushParams, SMALL_PALM_BUSH);
remixed_sbs_plant!(JuvenilePalmBush, PalmBush, PalmBushParams, JUVENILE_PALM_BUSH);

#[derive(Clone, Debug, Args)]
#[command(rename_all = "kebab-case")]
pub struct TropicalTuftsParams {
	#[command(flatten)]
	pub preview: GrovePreviewParams<TropicalTuftsCell>,

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

impl Default for TropicalTuftsParams {
	fn default() -> Self {
		Self {
			preview: GrovePreviewParams::default().with_terrain(FlatTerrainSample::default()),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			merge_collections: 0,
			patch_variants: 100,
		}
	}
}

crate::impl_grove_preview_params!(TropicalTuftsParams, TropicalTuftsCell);

impl TropicalTuftsParams {
	// preview accessors via impl_grove_preview_params!
	pub fn build(&self) -> TropicalTufts {
		self.build_on(&self.terrain)
	}

	/// Grow placements against `world` ([`crate::GroveWorldSample::height_at`]).
	pub fn build_on(&self, world: &impl crate::GroveWorldSample) -> TropicalTufts {
		let foliage_noise = self.foliage_noise;
		let variants = self.patch_variants.max(1);
		let mut tuft_grown = Vec::new();
		let mut palms = Vec::new();
		for placed in self.placements_on(world) {
			let mix = placed.variant.palette_mix();
			match placed.variant {
				TropicalTuftsCell::SmallPalmBush | TropicalTuftsCell::JuvenilePalmBush => {
					let variant = patch_variant_index(placed.position, variants);
					let (bush, world_size) = match placed.variant {
						TropicalTuftsCell::SmallPalmBush => SmallPalmBush::grow_num(variant),
						TropicalTuftsCell::JuvenilePalmBush => JuvenilePalmBush::grow_num(variant),
						_ => unreachable!("palm cells only"),
					};
					let material = material_from_palette(mix, placed.position, foliage_noise);
					let frond_material = frond_material_from_palette(
						Some(mix),
						placement_noise(foliage_noise, placed.position).seed,
					);
					palms.push(TropicalTuftsPalm {
						placement: Placement::new(placed.position, 0.0)
							.with_scale(Vec3::splat((placed.scale * world_size).max(1e-4))),
						bush,
						material,
						frond_material,
					});
				}
				cell => {
					let variant = patch_variant_index(placed.position, variants);
					let (patch, world_size) = match cell {
						TropicalTuftsCell::BrightTuft => BrightTuft::grow_num(variant),
						TropicalTuftsCell::DeepTuft => DeepTuft::grow_num(variant),
						TropicalTuftsCell::YellowGreenTuft => YellowGreenTuft::grow_num(variant),
						TropicalTuftsCell::BrightTuftPatch => BrightTuftPatch::grow_num(variant),
						TropicalTuftsCell::DeepTuftPatch => DeepTuftPatch::grow_num(variant),
						TropicalTuftsCell::YellowGreenTuftPatch => {
							YellowGreenTuftPatch::grow_num(variant)
						}
						TropicalTuftsCell::SmallPalmBush | TropicalTuftsCell::JuvenilePalmBush => {
							unreachable!("palm cells handled above")
						}
					};
					let material = material_from_palette(mix, placed.position, foliage_noise);
					tuft_grown.push(unit_plant_from_grown(
						patch,
						world_size,
						placed.position,
						placed.scale,
						material,
					));
				}
			}
		}
		TropicalTufts {
			body: TuftGroveBody::from_plants(
				grow_tuft_plants(tuft_grown, self.merge_collections, &self.extent),
				&self.extent,
				self.cell_extent_xz(),
				TuftGroveProxyHeights::SHORT,
			),
			palms: palms.into(),
		}
	}
}

#[derive(Clone)]
struct TropicalTuftsPalm {
	placement: Placement,
	bush: Arc<PalmBush>,
	material: MaterialRef,
	frond_material: MaterialRef,
}

#[derive(Clone, Component)]
pub struct TropicalTufts {
	body: TuftGroveBody,
	palms: Arc<[TropicalTuftsPalm]>,
}

impl TropicalTufts {
	pub fn plants(&self) -> &[TuftGrovePlant] {
		self.body.plants()
	}

	pub fn palm_count(&self) -> usize {
		self.palms.len()
	}

	/// High tuft kits with a height floor, plus authored palms (forest present dropout).
	pub fn high_chunks_dropping_shorter_than(
		&self,
		lod_ref: &LodRef,
		min_height_m: f32,
	) -> SceneChunk {
		let tufts = self.body.high_medium_chunks_dropping_shorter_than(
			lod_ref,
			LodSceneLevel::High,
			min_height_m,
		);
		if self.palms.is_empty() {
			return tufts;
		}
		SceneChunk::chunks([tufts, self.lazy_palm_chunks(lod_ref, LodSceneLevel::High)])
	}

	#[cfg(test)]
	pub(crate) fn palms_share_unit_arc(&self) -> bool {
		self.palms.len() >= 2 && Arc::ptr_eq(&self.palms[0].bush, &self.palms[1].bush)
	}

	fn tuft_scene_chunks(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				let tufts = self.body.high_medium_chunks(lod_ref, level);
				if self.palms.is_empty() {
					return tufts;
				}
				let palm_level = match level {
					LodSceneLevel::Medium => LodSceneLevel::High,
					other => other,
				};
				SceneChunk::chunks([tufts, self.lazy_palm_chunks(lod_ref, palm_level)])
			}
			_ => {
				let tufts = self.body.low_ultra_chunks(lod_ref, level);
				if self.palms.is_empty() {
					return tufts;
				}
				SceneChunk::chunks([tufts, self.lazy_palm_chunks(lod_ref, level)])
			}
		}
	}

	fn lazy_palm_chunks(&self, lod_ref: &LodRef, palm_level: LodSceneLevel) -> SceneChunk {
		use vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT;

		let palms = Arc::clone(&self.palms);
		let n: usize = palms
			.iter()
			.map(|palm| palm.bush.foliage_nodes_for_level(palm_level).flatten().len())
			.sum();
		if n == 0 {
			return SceneChunk::primitive(vegetation_components::scene_children(Vec::new()));
		}
		let prev = *lod_ref.previous_transform;
		let curr = *lod_ref.current_transform;
		let bounds = *lod_ref.bounds;
		let entity = lod_ref.entity;
		let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
		let mut palm_index = 0usize;
		let mut pending: Vec<FoliageNode> = Vec::new();
		SceneChunk::lazy(n as u32 * kit_w, n, move || {
			let kit_lod = LodRef {
				entity,
				previous_transform: &prev,
				current_transform: &curr,
				bounds: &bounds,
			};
			loop {
				if let Some(node) = pending.pop() {
					return Some(SceneChunk::weighted(
						kit_w,
						node.scene_with_level(&kit_lod, palm_level),
					));
				}
				if palm_index >= palms.len() {
					return None;
				}
				let palm = &palms[palm_index];
				palm_index += 1;
				pending = flatten_foliage_nodes(
					palm.bush.as_ref(),
					palm.placement,
					&palm.material,
					&palm.frond_material,
					palm_level,
				);
				pending.reverse();
			}
		})
	}
}

impl VegetationComponents for TropicalTufts {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		tuft_grove_stick_nodes(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let mut nodes = self.body.foliage_for_level(level).flatten();
		let palm_level = match level {
			LodSceneLevel::Medium => LodSceneLevel::High,
			other => other,
		};
		for palm in self.palms.iter() {
			nodes.extend(flatten_foliage_nodes(
				palm.bush.as_ref(),
				palm.placement,
				&palm.material,
				&palm.frond_material,
				palm_level,
			));
		}
		Layers::from_free(nodes)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.body.structural_lod())
	}
}

crate::impl_tuft_grove_lod_emit!(TropicalTufts);
