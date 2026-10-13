//! Shared authored pocket-water stamp enum for Watershed high/low passes.

use crate::terrain::watersheds::leaf_kind::WatershedLeafKind;
use terrain_watersheds::{Bog, HydroNode, Lake, Stream, StreamsGraph};

/// Authored Watershed pocket-water stamp held by a pass leaf (not a compiled complex).
#[derive(Debug, Clone)]
pub enum PocketWater {
	Empty,
	Stream(Stream),
	StreamsGraph(StreamsGraph),
	Bog(Bog),
	Lake(Lake),
}

impl PocketWater {
	pub fn kind(&self) -> WatershedLeafKind {
		match self {
			Self::Empty => WatershedLeafKind::Empty,
			Self::Stream(_) => WatershedLeafKind::Stream,
			Self::StreamsGraph(_) => WatershedLeafKind::StreamsGraph,
			Self::Bog(_) => WatershedLeafKind::Bog,
			Self::Lake(_) => WatershedLeafKind::Lake,
		}
	}

	/// Hydrology nodes from this authored stamp (empty when unoccupied).
	pub fn hydro_nodes(&self) -> Vec<HydroNode> {
		match self {
			Self::Empty => Vec::new(),
			Self::Stream(s) => s.hydro_nodes(),
			Self::StreamsGraph(g) => g.hydro_nodes(),
			Self::Bog(b) => b.hydro_nodes(),
			Self::Lake(l) => l.hydro_nodes(),
		}
	}

	pub fn is_empty(&self) -> bool {
		matches!(self, Self::Empty)
	}
}
