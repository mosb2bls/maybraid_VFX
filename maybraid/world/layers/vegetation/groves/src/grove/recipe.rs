//! Selection surface for a grove without growing plants.
//!
//! A forest blended tile walks the presenting tile's cells and calls
//! [`GroveRecipe::select_cell`] on the winning neighbor recipe. Neighbor tiles
//! are not grown.

use bevy_math::Vec2;
use gimme_gen::Cell;

use super::extent::GroveExtent;
use super::terrain::GroveWorldSample;
use super::{Grove, GroveCellOutcome};

/// Cheap grove identity used at selection time (no `grow_num`, no `LodScene`).
pub trait GroveRecipe<V> {
	fn cell_extent_xz(&self) -> Vec2;

	/// World-aligned planting cells this presenting tile owns.
	fn cells_overlapping(&self, extent: &GroveExtent) -> Vec<Cell>;

	/// Place or reject one cell against the **presenting** tile footprint.
	fn select_cell(
		&self,
		cell: &Cell,
		extent: &GroveExtent,
		world: &impl GroveWorldSample,
	) -> GroveCellOutcome<V>;
}

impl<V: Clone> GroveRecipe<V> for Grove<V> {
	fn cell_extent_xz(&self) -> Vec2 {
		Grove::cell_extent_xz(self)
	}

	fn cells_overlapping(&self, extent: &GroveExtent) -> Vec<Cell> {
		extent.cells_overlapping(self.cell_extent_xz())
	}

	fn select_cell(
		&self,
		cell: &Cell,
		extent: &GroveExtent,
		world: &impl GroveWorldSample,
	) -> GroveCellOutcome<V> {
		Grove::select_cell(self, cell, extent, world)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::{
		FlatTerrainSample, ForestGroveBiases, GroveBucket, GroveDefinition, GroveDistribution,
		GrovePlacementRanges, PlacementConstraints,
	};
	use anyhow::Result;
	use bevy_math::Vec3;
	use procedural_common::{NoiseParams, UnitRange};

	fn grove() -> Grove<&'static str> {
		Grove::assemble(
			GroveDefinition {
				cell_extent_xz: Vec2::splat(10.0),
				placement: GrovePlacementRanges::new(
					UnitRange::new(1.0, 1.0),
					UnitRange::new(0.0, 0.0),
				),
				distribution: GroveDistribution::new(vec![GroveBucket::placed(
					1.0,
					PlacementConstraints::UNCONSTRAINED,
					"tree",
				)]),
			},
			ForestGroveBiases::default(),
			NoiseParams::default(),
			Vec3::ZERO,
		)
	}

	#[test]
	fn recipe_cells_match_extent_overlap() -> Result<()> {
		let grove = grove();
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
		assert_eq!(
			GroveRecipe::cells_overlapping(&grove, &extent).len(),
			extent.cells_overlapping(grove.cell_extent_xz()).len()
		);
		let cells = GroveRecipe::cells_overlapping(&grove, &extent);
		let world = FlatTerrainSample { elevation: 0.2, steepness: 0.0 };
		let placed = GroveRecipe::select_cell(&grove, &cells[0], &extent, &world);
		assert!(matches!(placed, GroveCellOutcome::Placed { .. }));
		Ok(())
	}
}
