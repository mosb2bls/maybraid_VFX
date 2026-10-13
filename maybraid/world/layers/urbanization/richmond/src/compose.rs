//! Inner cells that development pads can be composed into.

use durham::Terrain;

use crate::{PadComplex, TerrainWithPads};

/// An inner model's cell that accepts pad elevation ops.
pub trait PadComposable {
	type Padded;

	fn compose_pads(&self, pads: &PadComplex) -> Self::Padded;
}

impl PadComposable for Terrain {
	type Padded = TerrainWithPads;

	fn compose_pads(&self, pads: &PadComplex) -> TerrainWithPads {
		TerrainWithPads::compose(self, std::iter::once(pads))
	}
}
