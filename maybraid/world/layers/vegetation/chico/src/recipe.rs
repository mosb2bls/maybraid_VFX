//! Type-erased [`GroveRecipe`](vegetation_groves::GroveRecipe) for [`ForestGroveKind`].
//!
//! Selection uses the presenting tile's cells. Grow only the placements that won.

use bevy::math::bounding::Aabb3d;
use bevy_math::Vec2;
use gimme_gen::Cell;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk};
use vegetation_groves::{
	AlpineParams, AridConiferSaplingParams, BraidGrassParams, BushScrubParams,
	ChristmasTaigaParams, CommonTuftsParams, ConiferMassivesParams, ConiferSaplingParams,
	DateGroveParams, DrylandParams, ForlornSavannaParams, GoettingenFollowParams, GroveExtent,
	GroveFrontend, GroveRecipe, GroveWorldSample, HighBushParams, JerrysChaparralParams,
	JungleLowerMassivesParams, JungleMassivesParams, LeewardParams, LevantineScrubParams,
	LowBushParams, MonsterGrassParams, OrchardParams, PalmShadeParams, RiparianGeneralParams,
	RiparianMixParams, RiverineGreenParams, RollingOaksParams, ShamanhomeParams,
	SpottyBushesParams, StorytellersParams, StrangeOasisParams, TallGrassParams,
	TemperateLowerMassivesParams, TemperateMassivesParams, TradeWindsParams, TropicalThicketParams,
	TropicalTuftsParams, TropicalUndergrowthParams, UnendingJungleParams, VineyardParams,
	WanderingAcaciaParams, WildGrassParams,
};

use crate::{ForestGroveKind, ForestGroveTile};

/// Unique woody archetypes for assembled world/forest tiles.
///
/// Authored grove previews keep [`vegetation_groves::GrovePreviewParams::default`]
/// (`100`). Runtime world diversity is a separate setting.
pub const WORLD_FOREST_TREE_VARIANTS: u32 = 32;

/// Blend result for one presenting tile: kind plus the cells that won.
///
/// `cells` is `None` when the tile is uniform (grow the whole footprint).
#[derive(Clone)]
pub struct ForestGroveRecipe {
	pub kind: ForestGroveKind,
	pub extent: GroveExtent,
	pub cells: Option<Vec<Cell>>,
}

impl ForestGroveRecipe {
	pub fn uniform(kind: ForestGroveKind, extent: GroveExtent) -> Self {
		Self { kind, extent, cells: None }
	}

	pub fn on_cells(kind: ForestGroveKind, extent: GroveExtent, cells: Vec<Cell>) -> Self {
		Self { kind, extent, cells: Some(cells) }
	}

	pub fn grow(&self, world: &impl GroveWorldSample) -> ForestGroveTile {
		match &self.cells {
			None => self.kind.grow_tile(self.extent, world),
			Some(cells) => self.kind.grow_on_cells(self.extent, cells, world),
		}
	}
}

macro_rules! impl_kind_recipe {
	($($Kind:ident => $mod:ident, $Params:ident),+ $(,)?) => {
		impl ForestGroveKind {
			/// Authored planting-cell span for this grove.
			pub fn cell_extent_xz(self) -> Vec2 {
				match self {
					$(Self::$Kind => vegetation_groves::$mod::definition().cell_extent_xz,)+
				}
			}

			/// Grow the full tile with default params (no construction-seed bias).
			///
			/// Caps woody archetypes at [`WORLD_FOREST_TREE_VARIANTS`]. Authored
			/// grove previews keep [`vegetation_groves::GrovePreviewParams::default`].
			pub fn grow_tile(
				self,
				extent: GroveExtent,
				world: &impl GroveWorldSample,
			) -> ForestGroveTile {
				match self {
					$(Self::$Kind => ForestGroveTile::$Kind(
						$Params::default()
							.with_extent(extent)
							.with_tree_variants(WORLD_FOREST_TREE_VARIANTS)
							.build_on(world),
					),)+
				}
			}

			/// `select_cell` on `cells`, then grow those placements on `extent`.
			pub fn grow_on_cells(
				self,
				extent: GroveExtent,
				cells: &[Cell],
				world: &impl GroveWorldSample,
			) -> ForestGroveTile {
				match self {
					$(Self::$Kind => {
						let grove = GroveFrontend::default()
							.assemble(vegetation_groves::$mod::definition());
						let placements = cells
							.iter()
							.filter_map(|cell| {
								GroveRecipe::select_cell(&grove, cell, &extent, world).into_placed()
							})
							.collect();
						let mut params = $Params::default()
							.with_extent(extent)
							.with_tree_variants(WORLD_FOREST_TREE_VARIANTS);
						params.preview = params.preview.clone().with_resolved_placements(placements);
						ForestGroveTile::$Kind(params.build_on(world))
					})+
				}
			}
		}

		/// Match a [`ForestGroveTile`] and bind the concrete grove as `$g`.
		#[macro_export]
		macro_rules! match_forest_grove_tile {
			($tile:expr, $g:ident => $body:expr) => {
				match $tile {
					$($crate::ForestGroveTile::$Kind($g) => $body,)+
				}
			};
		}
	};
}

impl_kind_recipe! {
	Alpine => alpine, AlpineParams,
	AridConiferSapling => arid_conifer_sapling, AridConiferSaplingParams,
	BraidGrass => braid_grass, BraidGrassParams,
	BushScrub => bush_scrub, BushScrubParams,
	ChristmasTaiga => christmas_taiga, ChristmasTaigaParams,
	CommonTufts => common_tufts, CommonTuftsParams,
	ConiferMassives => conifer_massives, ConiferMassivesParams,
	ConiferSapling => conifer_sapling, ConiferSaplingParams,
	DateGrove => date_grove, DateGroveParams,
	Dryland => dryland, DrylandParams,
	ForlornSavanna => forlorn_savanna, ForlornSavannaParams,
	GoettingenFollow => goettingen_follow, GoettingenFollowParams,
	HighBush => high_bush, HighBushParams,
	JerrysChaparral => jerrys_chaparral, JerrysChaparralParams,
	JungleLowerMassives => jungle_lower_massives, JungleLowerMassivesParams,
	JungleMassives => jungle_massives, JungleMassivesParams,
	Leeward => leeward, LeewardParams,
	LevantineScrub => levantine_scrub, LevantineScrubParams,
	LowBush => low_bush, LowBushParams,
	MonsterGrass => monster_grass, MonsterGrassParams,
	Orchard => orchard, OrchardParams,
	PalmShade => palm_shade, PalmShadeParams,
	RiparianGeneral => riparian_general, RiparianGeneralParams,
	RiparianMix => riparian_mix, RiparianMixParams,
	RiverineGreen => riverine_green, RiverineGreenParams,
	RollingOaks => rolling_oaks, RollingOaksParams,
	Shamanhome => shamanhome, ShamanhomeParams,
	SpottyBushes => spotty_bushes, SpottyBushesParams,
	Storytellers => storytellers, StorytellersParams,
	StrangeOasis => strange_oasis, StrangeOasisParams,
	TallGrass => tall_grass, TallGrassParams,
	TemperateLowerMassives => temperate_lower_massives, TemperateLowerMassivesParams,
	TemperateMassives => temperate_massives, TemperateMassivesParams,
	TradeWinds => trade_winds, TradeWindsParams,
	TropicalThicket => tropical_thicket, TropicalThicketParams,
	TropicalTufts => tropical_tufts, TropicalTuftsParams,
	TropicalUndergrowth => tropical_undergrowth, TropicalUndergrowthParams,
	UnendingJungle => unending_jungle, UnendingJungleParams,
	Vineyard => vineyard, VineyardParams,
	WanderingAcacia => wandering_acacia, WanderingAcaciaParams,
	WildGrass => wild_grass, WildGrassParams,
}

impl ForestGroveTile {
	/// Blade / tuft-patch groves (High kits, not nested woody plants).
	pub fn is_tuft(&self) -> bool {
		matches!(
			self,
			Self::BraidGrass(_)
				| Self::CommonTufts(_)
				| Self::MonsterGrass(_)
				| Self::TallGrass(_)
				| Self::TropicalTufts(_)
				| Self::WildGrass(_)
		)
	}

	pub fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		match_forest_grove_tile!(self, g => g.scene_lod_level(lod_ref))
	}

	pub fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match_forest_grove_tile!(self, g => g.scene_lod_status(lod_ref))
	}

	pub fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		match_forest_grove_tile!(self, g => g.scene_lod_culls(lod_ref, current))
	}

	pub fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match_forest_grove_tile!(self, g => g.scene_chunks_with_level(lod_ref, level))
	}

	pub fn scene_bounds(&self) -> Aabb3d {
		match_forest_grove_tile!(self, g => g.scene_bounds())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;
	use vegetation_groves::{cell_center, FlatTerrainSample, GroveExtent, OrchardParams};

	#[test]
	fn orchard_cell_extent_matches_definition() -> Result<()> {
		assert_eq!(ForestGroveKind::Orchard.cell_extent_xz(), Vec2::splat(11.0));
		Ok(())
	}

	#[test]
	fn grow_on_cells_uses_only_those_cells() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let cells = extent.cells_overlapping(ForestGroveKind::Orchard.cell_extent_xz());
		let one = cells.first().cloned().ok_or_else(|| anyhow::anyhow!("expected a cell"))?;
		let tile = ForestGroveKind::Orchard.grow_on_cells(
			extent,
			std::slice::from_ref(&one),
			&FlatTerrainSample::default(),
		);
		assert!(matches!(tile, ForestGroveTile::Orchard(_)));
		let _ = cell_center(&one);
		Ok(())
	}

	#[test]
	fn world_forest_tree_variants_stay_below_authored_preview_default() {
		assert_eq!(WORLD_FOREST_TREE_VARIANTS, 32);
		let preview = vegetation_groves::GrovePreviewParams::<()>::default();
		assert_eq!(preview.tree_variants, 100);
		assert!(WORLD_FOREST_TREE_VARIANTS < preview.tree_variants);
		let params = OrchardParams::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0)))
			.with_tree_variants(WORLD_FOREST_TREE_VARIANTS);
		assert_eq!(params.tree_variants, WORLD_FOREST_TREE_VARIANTS);
	}
}
