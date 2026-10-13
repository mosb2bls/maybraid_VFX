//! Common grove selection container ([RFC-183 §4.7], [#192](https://github.com/ramate-io/maybraid/issues/192)).
//!
//! A grove is an authored [`GroveDefinition`] (cell footprint, placement ranges, weighted
//! variant distribution) assembled with parent-forest biases and shared noise into a [`Grove`]
//! that deterministically selects and places vegetation per cell:
//!
//! 1. sample per-cell scale and offset from authored ranges ([RFC-183 3.4.1]),
//! 2. resolve the candidate position against the presenting tile ([RFC-183 3.4.2.3]),
//! 3. sample terrain elevation at the resolved point ([RFC-183 3.4.2.4]),
//! 4. bucket-throw plus ordered first-fit over the distribution ([RFC-183 3.4.2.5]).
//!
//! v1 exposes direct assembly constructors; [`gimme_gen::CellGenerator`] integration follows
//! once the spatial index lands in gimme.

mod distribution;
mod extent;
mod frontend;
mod palette;
mod recipe;
mod sampling;
mod terrain;
mod tuft_patch;

#[cfg(feature = "render")]
#[allow(dead_code)]
mod placed_host;
#[cfg(feature = "render")]
mod preview;
#[cfg(feature = "render")]
mod quantized;
#[cfg(feature = "render")]
mod tuft_lod;
#[cfg(feature = "render")]
pub mod vc_compose;
#[cfg(feature = "render")]
pub mod vc_tuft;
#[cfg(all(test, feature = "render"))]
pub(crate) mod woody_checks;
#[cfg(feature = "render")]
mod woody_lod;

pub use distribution::{
	parse_variant_weights, GroveBucket, GroveDistribution, PreparedGroveDistribution,
	VariantWeightOverrides,
};
pub use extent::{GroveExtent, DEFAULT_GROVE_EXTENT_XZ};
pub use frontend::{parse_vec2_csv, parse_vec3_csv, GroveFrontend};
pub use palette::{PaletteColor, PaletteMix, PaletteSlot};
pub use recipe::GroveRecipe;
pub use sampling::{
	cell_center, placement_noise, ForestGroveBiases, GrovePlacementRanges, PlacementSample,
};
pub use terrain::{
	FlatTerrainSample, FnHeightSample, GroveHeightModulation, GroveHeightModulationStack,
	GroveTerrain, GroveWorldSample, ModulatedGroveSample, PlacementConstraints, TerrainGroveSample,
};
pub use tuft_patch::GroveTuftPatch;

#[cfg(feature = "render")]
pub use palette::{patch_spawned_leaf_material, resolve_palette_color, WithPalette};
#[cfg(feature = "render")]
pub use preview::GrovePreviewParams;
#[cfg(feature = "render")]
pub(crate) use quantized::{
	remixed_blade_tuft_plant, remixed_bush_plant, remixed_sbs_plant, remixed_spear_tuft_plant,
	remixed_tuft_plant,
};
#[cfg(feature = "render")]
pub use quantized::{unit_build_noise, unit_chain_noise};
#[cfg(feature = "render")]
pub use vc_compose::{
	canopy_ball_material_from_palette, canopy_proxy_column, canopy_proxy_crown, canopy_proxy_rory,
	canopy_proxy_site, canopy_proxy_site_nested, canopy_proxy_trained, canopy_proxy_trunk,
	canopy_proxy_waialea, flatten_foliage_nodes, flatten_foliage_nodes_nested, flatten_stick_nodes,
	flattened_plant_lazy_weight, foliage_low_canopy_balls, foliage_ultra_low_merged_balls,
	frond_material_from_palette, grove_bands_for_typical_height,
	grove_bands_for_typical_height_and_plant_medium, grove_detail_level,
	grove_detail_level_keep_low, grove_lod_culls, grove_lod_level, grove_lod_status,
	grove_structural_footprint, layers_from_nodes, lazy_flattened_plant_chunks,
	nest_flattened_plant_chunk, nest_flattened_plant_host, placed_foliage_nodes,
	placed_palm_low_fronds, stick_material_from_palette, trained_proxy_stick_nodes_for_level,
	woody_grove_scene_chunks, woody_grove_scene_chunks_keep_low_plants, CanopyProxySite,
	TrainedCanopyProxy, DEFAULT_PLANT_MEDIUM_FACTOR, ULTRA_LOW_CANOPY_BIN_METERS,
};
#[cfg(feature = "render")]
pub use vc_tuft::{remixed_blade_tuft_unit, remixed_spear_tuft_unit, remixed_tuft_unit};
#[cfg(feature = "render")]
pub use woody_lod::{WoodyCanopyPolicy, WoodyGroveLod};

use bevy_math::{Vec2, Vec3};
use gimme_gen::Cell;
use procedural_common::NoiseParams;

/// Authored grove identity: cell footprint, per-cell placement ranges, and variant distribution.
///
/// Typically, this will be requested first by the generation hierarchy.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveDefinition<V> {
	/// Vegetation cell span in world metres on X and Z.
	pub cell_extent_xz: Vec2,
	/// Ranges sampled independently for each cell draw.
	pub placement: GrovePlacementRanges,
	/// Ordered weighted variants, including the explicit `None` bucket.
	pub distribution: GroveDistribution<V>,
}

/// Assembled grove with forest biases, shared noise, and a pre-perturbed distribution.
///
/// This will be assembled from a definition by the generation hierarchy.
#[derive(Debug, Clone)]
pub struct Grove<V> {
	cell_extent_xz: Vec2,
	placement: GrovePlacementRanges,
	biases: ForestGroveBiases,
	noise: NoiseParams,
	distribution: PreparedGroveDistribution<V>,
}

/// One placed grove item ready for materialization.
///
/// This is the result of the selection and generation pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveCellVariant<V> {
	/// The variant that was placed.
	pub variant: V,
	/// The position that was placed.
	pub position: Vec3,
	/// The scale that was placed.
	pub scale: f32,
}

impl<V> GroveCellVariant<V> {
	pub fn new(variant: V, position: Vec3, scale: f32) -> Self {
		Self { variant, position, scale }
	}
}

/// Result of running the full selection pipeline on one grove cell.
#[derive(Debug, Clone, PartialEq)]
pub enum GroveCellOutcome<V> {
	Placed {
		variant: V,
		position: Vec3,
		scale: f32,
	},
	/// Explicit `None` bucket won first-fit at this candidate point. The position is the
	/// evaluated placement (cell center + offset) so empty outcomes stay addressable in space
	/// and remain stable across chunk reloads.
	Empty {
		position: Vec3,
	},
	/// The candidate fell outside the presenting tile, or every bucket failed placement
	/// constraints. The position records where validation ran so callers can debug terrain
	/// mismatch without inventing a fallback location.
	Rejected {
		position: Vec3,
	},
}

impl<V> GroveCellOutcome<V> {
	pub fn into_placed(self) -> Option<GroveCellVariant<V>> {
		match self {
			GroveCellOutcome::Placed { variant, position, scale } => {
				Some(GroveCellVariant { variant, position, scale })
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => None,
		}
	}
}

impl<V: Clone> Grove<V> {
	/// Assemble a grove and perturb bucket weights once at `perturbation_origin`.
	pub fn assemble(
		definition: GroveDefinition<V>,
		biases: ForestGroveBiases,
		noise: NoiseParams,
		perturbation_origin: Vec3,
	) -> Self {
		let distribution = definition.distribution.prepare(
			biases.bucket_mean_shift,
			biases.bucket_perturbation_bias,
			noise,
			perturbation_origin,
		);
		Self {
			cell_extent_xz: definition.cell_extent_xz,
			placement: definition.placement,
			biases,
			noise,
			distribution,
		}
	}

	/// World-aligned cells this tile owns, then select a placement for each one.
	pub fn populate(
		&self,
		extent: &GroveExtent,
		world: &impl GroveWorldSample,
	) -> Vec<GroveCellVariant<V>> {
		self.cells_overlapping(extent)
			.iter()
			.filter_map(|cell| GroveRecipe::select_cell(self, cell, extent, world).into_placed())
			.collect()
	}

	/// Sample, place, validate, and choose a bucket for one vegetation cell.
	pub fn select_cell(
		&self,
		cell: &Cell,
		extent: &GroveExtent,
		world: &impl GroveWorldSample,
	) -> GroveCellOutcome<V> {
		let sample = self.placement.sample_cell(&self.biases, self.noise, cell);
		let candidate = sample.position_in(cell);
		// Cell offsets may overspill the cell, but stay on the presenting tile.
		if !extent.contains_xz(candidate) {
			return GroveCellOutcome::Rejected { position: candidate };
		}
		let position = Vec3::new(candidate.x, world.height_at(candidate), candidate.z);
		if !world.allows_placement_at(position) {
			return GroveCellOutcome::Rejected { position };
		}
		self.distribution.select_at(position, sample.scale, *cell, self.noise, world)
	}

	pub fn cell_extent_xz(&self) -> Vec2 {
		self.cell_extent_xz
	}

	pub fn placement_ranges(&self) -> GrovePlacementRanges {
		self.placement
	}

	pub fn biases(&self) -> &ForestGroveBiases {
		&self.biases
	}

	pub fn noise(&self) -> NoiseParams {
		self.noise
	}

	pub fn distribution(&self) -> &PreparedGroveDistribution<V> {
		&self.distribution
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::UnitRange;

	fn test_definition() -> GroveDefinition<&'static str> {
		GroveDefinition {
			cell_extent_xz: Vec2::splat(10.0),
			placement: GrovePlacementRanges::new(
				UnitRange::new(0.8, 1.2),
				UnitRange::new(-0.2, 0.2),
			),
			distribution: GroveDistribution::new(vec![GroveBucket::placed(
				1.0,
				PlacementConstraints::UNCONSTRAINED,
				"tree",
			)]),
		}
	}

	fn flat(elevation: f32, steepness: f32) -> FlatTerrainSample {
		FlatTerrainSample { elevation, steepness }
	}

	fn test_extent() -> GroveExtent {
		GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0))
	}

	#[test]
	fn populate_places_variants_at_terrain_elevation() -> Result<()> {
		let grove = Grove::assemble(
			test_definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let placements = grove.populate(&test_extent(), &flat(0.4, 0.1));
		assert!(!placements.is_empty());
		for placed in &placements {
			assert_eq!(placed.variant, "tree");
			assert!((placed.position.y - 0.4).abs() < 1e-6);
			assert!((0.8..=1.2).contains(&placed.scale));
		}
		Ok(())
	}

	#[test]
	fn candidate_outside_extent_is_rejected() -> Result<()> {
		let mut definition = test_definition();
		// Offsets always push the candidate 20 m outside the 10 m grove footprint.
		definition.placement =
			GrovePlacementRanges::new(UnitRange::new(1.0, 1.0), UnitRange::new(20.0, 20.0));
		let grove = Grove::assemble(
			definition,
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let extent = test_extent();
		let cells = grove.cells_overlapping(&extent);
		let outcome = grove.select_cell(&cells[0], &extent, &flat(0.4, 0.1));
		assert!(matches!(outcome, GroveCellOutcome::Rejected { .. }));
		assert!(grove.populate(&extent, &flat(0.4, 0.1)).is_empty());
		Ok(())
	}

	#[test]
	fn selection_is_deterministic() -> Result<()> {
		let grove = Grove::assemble(
			test_definition(),
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		);
		let a = grove.populate(&test_extent(), &flat(0.35, 0.15));
		let b = grove.populate(&test_extent(), &flat(0.35, 0.15));
		assert_eq!(a, b);
		Ok(())
	}
}
