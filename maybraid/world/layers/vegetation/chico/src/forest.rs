//! Chico forest model: Hopscotch a cell to selected grove kinds.
//!
//! This is a generation result, not a `LodScene` host. Growing plants is
//! [`crate::ChicoGrove`] / [`crate::assemble`], not this type.

use procedural_common::NoiseParams;

use crate::{select_cell, ForestExtent, LayeringKind, NeighborLayers, SelectedLayers};

/// One selected forest cell (extent + per-layer grove kinds).
#[derive(Clone, Debug)]
pub struct ChicoForest {
	pub extent: ForestExtent,
	pub layers: SelectedLayers,
}

impl ChicoForest {
	/// Hopscotch + per-layer Bucket Throw. Does not grow tiles.
	pub fn select_on(extent: ForestExtent, noise: NoiseParams) -> Self {
		Self { extent, layers: select_cell(extent, noise) }
	}

	/// Pin a well-known layering's typical (highest-weight) groves.
	pub fn select_layering(extent: ForestExtent, layering: LayeringKind) -> Self {
		Self { extent, layers: layering.layering().typical_layers() }
	}
}

/// Hopscotch the four cardinal neighbors (selection only).
pub fn neighbor_layers(extent: ForestExtent, noise: NoiseParams) -> NeighborLayers {
	let (ix, iz) = ForestExtent::cell_index_containing(extent.center());
	NeighborLayers {
		north: Some(select_cell(ForestExtent::from_cell_index(ix, iz + 1), noise)),
		east: Some(select_cell(ForestExtent::from_cell_index(ix + 1, iz), noise)),
		south: Some(select_cell(ForestExtent::from_cell_index(ix, iz - 1), noise)),
		west: Some(select_cell(ForestExtent::from_cell_index(ix - 1, iz), noise)),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;

	#[test]
	fn select_on_records_extent_and_layers() -> Result<()> {
		let extent = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let forest = ChicoForest::select_on(extent, NoiseParams::default());
		assert_eq!(forest.extent, extent);
		let selected = [
			forest.layers.tufts,
			forest.layers.understory,
			forest.layers.lower_canopy,
			forest.layers.upper_canopy,
		]
		.into_iter()
		.filter(Option::is_some)
		.count();
		assert!(selected <= 4);
		Ok(())
	}

	#[test]
	fn select_layering_uses_typical_lush_jungle_groves() -> Result<()> {
		let extent = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let forest = ChicoForest::select_layering(extent, LayeringKind::LushJungle);
		assert_eq!(forest.layers.layering, LayeringKind::LushJungle);
		assert!(forest.layers.upper_canopy.is_some());
		Ok(())
	}
}
