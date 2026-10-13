//! Pocket-water leaf typing for debug overlays and HUD.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Color;

/// Resolved Watershed stamp on a guillotine leaf (after empty fallbacks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatershedLeafKind {
	/// Occupancy miss or every stamp recipe skipped.
	Empty,
	Stream,
	/// Multi-corridor stream graph (composed soft-voronoi bands).
	StreamsGraph,
	Bog,
	Lake,
}

impl WatershedLeafKind {
	/// Wire color for playground / gizmo overlays.
	pub fn debug_color(self) -> Color {
		match self {
			Self::Empty => Color::srgba(0.55, 0.55, 0.6, 0.35),
			Self::Stream => Color::srgb(0.15, 0.85, 0.95),
			Self::StreamsGraph => Color::srgb(0.05, 0.65, 0.85),
			Self::Bog => Color::srgb(0.55, 0.75, 0.2),
			Self::Lake => Color::srgb(0.2, 0.45, 1.0),
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Empty => "empty",
			Self::Stream => "stream",
			Self::StreamsGraph => "streams_graph",
			Self::Bog => "bog",
			Self::Lake => "lake",
		}
	}
}

/// Which Watershed occupancy band produced a leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatershedBandPass {
	Low,
	High,
}

impl WatershedBandPass {
	pub fn label(self) -> &'static str {
		match self {
			Self::Low => "low",
			Self::High => "high",
		}
	}
}

/// Leaf AABB + stamp kind retained on [`crate::terrain::Terrain`] for debug.
#[derive(Debug, Clone, Copy)]
pub struct WatershedLeafBounds {
	pub cell: Aabb3d,
	pub kind: WatershedLeafKind,
	pub band: WatershedBandPass,
}
