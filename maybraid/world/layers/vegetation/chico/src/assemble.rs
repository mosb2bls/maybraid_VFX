//! Grow selected groves on 100 m tiles inside a forest cell.
//!
//! Each tile calls `Params::default().with_extent(tile).with_tree_variants(WORLD_FOREST_TREE_VARIANTS).build_on(world)`.
//! Construction noise stays on the grove default — forests do not bias `build_unit` seeds.

use bevy_math::Vec3;
use vegetation_groves::{GroveExtent, GroveWorldSample};

use crate::blend::{
	hash_unit_xz, neighbor_tile_steps, pick_kind, tile_center_xz, BlendSlot, Cardinal,
	GROVE_BLEND_RADIUS,
};
use crate::{ForestExtent, ForestGroveKind, ForestGroveRecipe, ForestLayer, SelectedLayers};

/// One grown grove tile (concrete grove type).
#[derive(Clone)]
pub enum ForestGroveTile {
	Alpine(vegetation_groves::Alpine),
	AridConiferSapling(vegetation_groves::AridConiferSapling),
	BraidGrass(vegetation_groves::BraidGrass),
	BushScrub(vegetation_groves::BushScrub),
	ChristmasTaiga(vegetation_groves::ChristmasTaiga),
	CommonTufts(vegetation_groves::CommonTufts),
	ConiferMassives(vegetation_groves::ConiferMassives),
	ConiferSapling(vegetation_groves::ConiferSapling),
	DateGrove(vegetation_groves::DateGrove),
	Dryland(vegetation_groves::Dryland),
	ForlornSavanna(vegetation_groves::ForlornSavanna),
	GoettingenFollow(vegetation_groves::GoettingenFollow),
	HighBush(vegetation_groves::HighBush),
	JerrysChaparral(vegetation_groves::JerrysChaparral),
	JungleLowerMassives(vegetation_groves::JungleLowerMassives),
	JungleMassives(vegetation_groves::JungleMassives),
	Leeward(vegetation_groves::Leeward),
	LevantineScrub(vegetation_groves::LevantineScrub),
	LowBush(vegetation_groves::LowBush),
	MonsterGrass(vegetation_groves::MonsterGrass),
	Orchard(vegetation_groves::Orchard),
	PalmShade(vegetation_groves::PalmShade),
	RiparianGeneral(vegetation_groves::RiparianGeneral),
	RiparianMix(vegetation_groves::RiparianMix),
	RiverineGreen(vegetation_groves::RiverineGreen),
	RollingOaks(vegetation_groves::RollingOaks),
	Shamanhome(vegetation_groves::Shamanhome),
	SpottyBushes(vegetation_groves::SpottyBushes),
	Storytellers(vegetation_groves::Storytellers),
	StrangeOasis(vegetation_groves::StrangeOasis),
	TallGrass(vegetation_groves::TallGrass),
	TemperateLowerMassives(vegetation_groves::TemperateLowerMassives),
	TemperateMassives(vegetation_groves::TemperateMassives),
	TradeWinds(vegetation_groves::TradeWinds),
	TropicalThicket(vegetation_groves::TropicalThicket),
	TropicalTufts(vegetation_groves::TropicalTufts),
	TropicalUndergrowth(vegetation_groves::TropicalUndergrowth),
	UnendingJungle(vegetation_groves::UnendingJungle),
	Vineyard(vegetation_groves::Vineyard),
	WanderingAcacia(vegetation_groves::WanderingAcacia),
	WildGrass(vegetation_groves::WildGrass),
}

/// Grown tiles for the four forest layers (ground cover stays empty).
#[derive(Clone)]
pub struct AssembledForest {
	pub layers: SelectedLayers,
	pub tufts: Vec<ForestGroveTile>,
	pub understory: Vec<ForestGroveTile>,
	pub lower_canopy: Vec<ForestGroveTile>,
	pub upper_canopy: Vec<ForestGroveTile>,
}

impl AssembledForest {
	/// Flattened tiles in layer order: tufts, understory, lower canopy, upper canopy.
	pub fn tiles(&self) -> impl Iterator<Item = &ForestGroveTile> {
		self.tufts
			.iter()
			.chain(self.understory.iter())
			.chain(self.lower_canopy.iter())
			.chain(self.upper_canopy.iter())
	}
}

/// Grow `kind` on one grove tile. Uses default grove params (no construction-seed bias).
pub fn grow_tile(
	kind: ForestGroveKind,
	extent: GroveExtent,
	world: &impl GroveWorldSample,
) -> ForestGroveTile {
	kind.grow_tile(extent, world)
}

fn grow_layer(
	kind: Option<ForestGroveKind>,
	tiles: &[GroveExtent],
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: ForestLayer,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	if kind.is_none() && !neighbors.any_kind(layer) {
		return Vec::new();
	}
	tiles
		.iter()
		.flat_map(|extent| grow_presenting_tile(kind, *extent, forest, neighbors, layer, world))
		.collect()
}

/// Adjacent forest-cell selections: the producer for grove slots those cells own.
///
/// Not grown. A presenting tile reads these when a neighbor grove was produced
/// outside this extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborLayers {
	pub north: Option<SelectedLayers>,
	pub east: Option<SelectedLayers>,
	pub south: Option<SelectedLayers>,
	pub west: Option<SelectedLayers>,
}

impl NeighborLayers {
	pub fn none() -> Self {
		Self { north: None, east: None, south: None, west: None }
	}

	pub fn any_kind(self, layer: ForestLayer) -> bool {
		[self.north, self.east, self.south, self.west]
			.into_iter()
			.flatten()
			.any(|layers| layer.kind(layers).is_some())
	}
}

/// Kind produced at a grove-slot center, or `None` if that producer is not cached.
fn produced_kind_at(
	center: Vec3,
	forest: ForestExtent,
	self_kind: Option<ForestGroveKind>,
	neighbors: &NeighborLayers,
	layer: ForestLayer,
) -> Option<Option<ForestGroveKind>> {
	if forest.owns_center_xz(center) {
		return Some(self_kind);
	}
	producer_layers(center, forest, neighbors).map(|selected| layer.kind(selected))
}

/// Adjacent-cell cache, or AABB overflow for 100 m test extents.
fn producer_layers(
	center: Vec3,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
) -> Option<SelectedLayers> {
	let self_idx = ForestExtent::cell_index_containing(forest.center());
	let slot_idx = ForestExtent::cell_index_containing(center);
	if slot_idx != self_idx {
		let (dx, dz) = (slot_idx.0 - self_idx.0, slot_idx.1 - self_idx.1);
		return match (dx, dz) {
			(1, 0) => neighbors.east,
			(-1, 0) => neighbors.west,
			(0, 1) => neighbors.north,
			(0, -1) => neighbors.south,
			_ => None,
		};
	}
	let east = center.x - forest.max().x;
	let west = forest.min().x - center.x;
	let north = center.z - forest.max().z;
	let south = forest.min().z - center.z;
	let over = east.max(west).max(north).max(south);
	if over < 0.0 {
		return None;
	}
	if over == east {
		neighbors.east
	} else if over == west {
		neighbors.west
	} else if over == north {
		neighbors.north
	} else {
		neighbors.south
	}
}

fn blend_sources(
	tile: GroveExtent,
	forest: ForestExtent,
	self_kind: Option<ForestGroveKind>,
	neighbors: &NeighborLayers,
	layer: ForestLayer,
) -> Vec<BlendSlot> {
	let mut slots = Vec::with_capacity(1 + 4 * GROVE_BLEND_RADIUS as usize);
	slots.push(BlendSlot { center: tile_center_xz(tile), kind: self_kind });
	for face in Cardinal::ALL {
		for step in 1..=GROVE_BLEND_RADIUS {
			let center = tile_center_xz(neighbor_tile_steps(tile, face, step));
			if let Some(kind) = produced_kind_at(center, forest, self_kind, neighbors, layer) {
				slots.push(BlendSlot { center, kind });
			}
		}
	}
	slots
}

/// Softmax blend for one presenting tile. Does not grow plants.
pub fn presenting_recipes(
	kind: Option<ForestGroveKind>,
	extent: GroveExtent,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: ForestLayer,
) -> Vec<ForestGroveRecipe> {
	let slots = blend_sources(extent, forest, kind, neighbors, layer);
	if !BlendSlot::has_planted(&slots) {
		return Vec::new();
	}
	if let Some(only) = BlendSlot::uniform_planted_kind(&slots) {
		return vec![ForestGroveRecipe::uniform(only, extent)];
	}
	recipe_blended(kind, extent, &slots)
}

fn grow_presenting_tile(
	kind: Option<ForestGroveKind>,
	extent: GroveExtent,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: ForestLayer,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	presenting_recipes(kind, extent, forest, neighbors, layer)
		.into_iter()
		.map(|recipe| recipe.grow(world))
		.collect()
}

fn recipe_blended(
	self_kind: Option<ForestGroveKind>,
	extent: GroveExtent,
	slots: &[BlendSlot],
) -> Vec<ForestGroveRecipe> {
	use std::collections::HashMap;

	use vegetation_groves::cell_center;

	let mut buckets: HashMap<ForestGroveKind, Vec<gimme_gen::Cell>> = HashMap::new();
	if let Some(self_kind) = self_kind {
		for cell in extent.cells_overlapping(self_kind.cell_extent_xz()) {
			let center = cell_center(&cell);
			if let Some(kind) = pick_kind(center, slots, hash_unit_xz(center)) {
				buckets.entry(kind).or_default().push(cell);
			}
		}
	} else {
		let mut lattices = Vec::<ForestGroveKind>::new();
		for lattice in BlendSlot::planted_kinds(slots) {
			if !lattices.contains(&lattice) {
				lattices.push(lattice);
			}
		}
		for lattice in lattices {
			for cell in extent.cells_overlapping(lattice.cell_extent_xz()) {
				let center = cell_center(&cell);
				if pick_kind(center, slots, hash_unit_xz(center)) == Some(lattice) {
					buckets.entry(lattice).or_default().push(cell);
				}
			}
		}
	}
	buckets
		.into_iter()
		.filter(|(_, cells)| !cells.is_empty())
		.map(|(kind, cells)| ForestGroveRecipe::on_cells(kind, extent, cells))
		.collect()
}

/// Assemble selected layers onto the forest cell's 100 m grove grid.
///
/// Every presenting tile softmax-blends a cardinal run of produced grove slots
/// (empty layers still grow neighbor islands). `neighbors` supply selections
/// for grove slots produced by adjacent forest cells.
pub fn assemble(
	extent: ForestExtent,
	layers: SelectedLayers,
	neighbors: NeighborLayers,
	world: &impl GroveWorldSample,
) -> AssembledForest {
	let tiles = extent.default_grove_tiles();
	AssembledForest {
		layers,
		tufts: grow_layer(layers.tufts, &tiles, extent, &neighbors, ForestLayer::Tufts, world),
		understory: grow_layer(
			layers.understory,
			&tiles,
			extent,
			&neighbors,
			ForestLayer::Understory,
			world,
		),
		lower_canopy: grow_layer(
			layers.lower_canopy,
			&tiles,
			extent,
			&neighbors,
			ForestLayer::LowerCanopy,
			world,
		),
		upper_canopy: grow_layer(
			layers.upper_canopy,
			&tiles,
			extent,
			&neighbors,
			ForestLayer::UpperCanopy,
			world,
		),
	}
}

/// Assemble with no neighbor recipes (same as isolated preview tiles).
pub fn assemble_isolated(
	extent: ForestExtent,
	layers: SelectedLayers,
	world: &impl GroveWorldSample,
) -> AssembledForest {
	assemble(extent, layers, NeighborLayers::none(), world)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{select_cell, ForestGroveKind, ForestLayer, LayeringKind, SelectedLayers};
	use anyhow::Result;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;
	use vegetation_groves::FlatTerrainSample;

	#[test]
	fn grow_one_ag_town_orchard_tile() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let tile = grow_tile(ForestGroveKind::Orchard, extent, &FlatTerrainSample::default());
		assert!(matches!(tile, ForestGroveTile::Orchard(_)));
		Ok(())
	}

	#[test]
	fn assemble_empty_layers_grows_nothing() -> Result<()> {
		let cell = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let grown = assemble(cell, layers, NeighborLayers::none(), &FlatTerrainSample::default());
		assert!(grown.tufts.is_empty());
		assert!(grown.upper_canopy.is_empty());
		let _ = select_cell(cell, NoiseParams::default());
		Ok(())
	}

	#[test]
	fn presenting_tile_blends_neighbor_recipe() -> Result<()> {
		let forest = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::AgTown,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::Orchard),
		};
		let neighbors = NeighborLayers {
			east: Some(SelectedLayers {
				layering: LayeringKind::MiRobles,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::RollingOaks),
			}),
			..NeighborLayers::none()
		};
		let grown = assemble(forest, layers, neighbors, &FlatTerrainSample::default());
		let kinds = grown
			.upper_canopy
			.iter()
			.map(|tile| match tile {
				ForestGroveTile::Orchard(_) => "orchard",
				ForestGroveTile::RollingOaks(_) => "oaks",
				_ => "other",
			})
			.collect::<Vec<_>>();
		assert!(kinds.contains(&"orchard"), "{kinds:?}");
		assert!(kinds.contains(&"oaks"), "expected a blended oak host, got {kinds:?}");
		Ok(())
	}

	#[test]
	fn interior_slot_uses_self_kind_not_far_forest() -> Result<()> {
		let forest = ForestExtent::default_cell();
		let tiles = forest.default_grove_tiles();
		let interior = tiles
			.iter()
			.copied()
			.find(|tile| {
				let c = (tile.min() + tile.max()) * 0.5;
				c.x.abs() <= 50.0 && c.z.abs() <= 50.0
			})
			.ok_or_else(|| anyhow::anyhow!("expected an origin-centered grove tile"))?;
		let neighbors = NeighborLayers {
			east: Some(SelectedLayers {
				layering: LayeringKind::MiRobles,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::RollingOaks),
			}),
			..NeighborLayers::none()
		};
		let slots = blend_sources(
			interior,
			forest,
			Some(ForestGroveKind::Orchard),
			&neighbors,
			ForestLayer::UpperCanopy,
		);
		let east = tile_center_xz(neighbor_tile_steps(interior, Cardinal::East, 1));
		let east_kind = slots.iter().find(|slot| {
			(slot.center.x - east.x).abs() < 1.0 && (slot.center.z - east.z).abs() < 1.0
		});
		let east_kind =
			east_kind.ok_or_else(|| anyhow::anyhow!("expected an east grove slot"))?.kind;
		assert_eq!(east_kind, Some(ForestGroveKind::Orchard));
		Ok(())
	}

	#[test]
	fn inward_tile_still_reads_far_grove_kind() -> Result<()> {
		let forest = ForestExtent::default_cell();
		let tiles = forest.default_grove_tiles();
		let inward = tiles
			.iter()
			.copied()
			.find(|tile| {
				let c = tile_center_xz(*tile);
				(c.x - 350.0).abs() < 60.0 && c.z.abs() <= 50.0
			})
			.ok_or_else(|| anyhow::anyhow!("expected a grove tile ~350 m east of origin"))?;
		let neighbors = NeighborLayers {
			east: Some(SelectedLayers {
				layering: LayeringKind::MiRobles,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::RollingOaks),
			}),
			..NeighborLayers::none()
		};
		let slots = blend_sources(
			inward,
			forest,
			Some(ForestGroveKind::Orchard),
			&neighbors,
			ForestLayer::UpperCanopy,
		);
		assert!(
			slots.iter().any(|slot| slot.kind == Some(ForestGroveKind::RollingOaks)),
			"radius should reach the east-produced oak groves from well inside the block"
		);
		Ok(())
	}

	#[test]
	fn empty_tile_grows_neighbor_islands() -> Result<()> {
		let forest = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let neighbors = NeighborLayers {
			west: Some(SelectedLayers {
				layering: LayeringKind::AgTown,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::Orchard),
			}),
			..NeighborLayers::none()
		};
		let grown = assemble(forest, layers, neighbors, &FlatTerrainSample::default());
		assert!(
			grown
				.upper_canopy
				.iter()
				.any(|tile| matches!(tile, ForestGroveTile::Orchard(_))),
			"empty layer should still present neighbor islands"
		);
		Ok(())
	}
}
