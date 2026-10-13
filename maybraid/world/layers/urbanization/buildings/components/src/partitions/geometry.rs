//! Continuous and tile partition geometry (primitive kit IR — no portals).

mod arc;
mod joint;
mod linear;
mod polyline;

pub use arc::ArcSweep;
pub use joint::{
	JointLod, JointPartition, JOINT_BASE_RADIUS, JOINT_HIGH_FACTOR, JOINT_KIT_HALF,
	JOINT_MEDIUM_FACTOR, JOINT_RADIUS_PER_SLOPE_RAD,
};
pub use linear::{
	fitted_tile_count, wall_placement, wall_placement_from_centered, LinearLod, LinearPartition,
	DEFAULT_THICK, DEFAULT_TILE_WIDTH, LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
	PANEL_TO_WALL_PITCH, PANEL_Y_HALF,
};
pub use polyline::{
	polyline_from_xz, roll_along_slope, PolylinePartition, DEFAULT_MIN_JOINT_ANGLE,
};

use bevy_math::Vec3;
use scene_ref::MirrorAxis;

use crate::arc_kit::ArcKit;
use crate::assets::partitions::rough_stonework::{
	ARC_15_HIGH, ARC_15_LOW, ARC_15_MID, ARC_180_HIGH, ARC_180_LOW, ARC_180_MID, ARC_90_HIGH,
	ARC_90_LOW, ARC_90_MID, LINEAR_HIGH, LINEAR_LOW, LINEAR_MID, SLICE_15_HIGH, SLICE_15_LOW,
	SLICE_15_MID, SLICE_90_HIGH, SLICE_90_LOW, SLICE_90_MID,
};
use crate::panels::{
	with_wall_standup_pitch, PanelGeometry, PanelKitCaps, Rectangle, RightTriangle,
	TessellatedTriangle,
};
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::style::PartitionStyle;
use crate::placed::{Placed, Placement};

/// Kit-local \(Y\) span of slice meshes (\([0, \texttt{SLICE_KIT_HEIGHT}]\)).
pub const SLICE_KIT_HEIGHT: f32 = 0.2;

/// Partition path geometry in world/cell space (continuous size and orientation).
#[derive(Debug, Clone, PartialEq)]
pub enum PartitionGeometry {
	Linear(LinearPartition),
	/// Circular joint tile (\(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\)).
	Joint(JointPartition),
	/// Short-run polyline (single LOD parent). Prefer splitting long paths upstream.
	Polyline(PolylinePartition),
	/// Arbitrary 3D triangle filled with right-triangle kits.
	TessellatedTriangle(TessellatedTriangle),
	Arc(ArcSweep),
	/// Slice-height arc (\(Y \in [0, [`SLICE_KIT_HEIGHT`]]\) in kit space).
	SliceArc(ArcSweep),
}

/// Alias for continuous partition geometry.
pub type Partition = PartitionGeometry;

impl Default for PartitionGeometry {
	fn default() -> Self {
		Self::linear()
	}
}

impl PartitionGeometry {
	pub fn linear() -> Self {
		Self::Linear(LinearPartition::default())
	}

	pub fn joint() -> Self {
		Self::Joint(JointPartition::default())
	}

	pub fn polyline(points: impl Into<Vec<Vec3>>) -> Self {
		Self::Polyline(PolylinePartition::new(points))
	}

	pub fn tessellated_triangle(t: TessellatedTriangle) -> Self {
		Self::TessellatedTriangle(t)
	}

	pub fn arc(sweep_degrees: f32) -> Self {
		Self::Arc(ArcSweep { sweep_degrees })
	}

	pub fn slice_arc(sweep_degrees: f32) -> Self {
		Self::SliceArc(ArcSweep { sweep_degrees })
	}

	/// Expand into posed leaf tiles (rough stonework style).
	pub fn tiles(&self) -> Vec<Placed<PartitionTile>> {
		self.tiles_for_style(PartitionStyle::RoughStonework)
	}

	/// Expand into posed leaf tiles under this geometry (identity parent).
	pub fn tiles_for_style(&self, style: PartitionStyle) -> Vec<Placed<PartitionTile>> {
		let panel_caps = PanelKitCaps::from(style);
		match self {
			Self::Linear(g) => g.tiles(),
			Self::Joint(_) => vec![Placed::at_origin(PartitionTile::Joint)],
			Self::Polyline(g) => g.tiles(),
			Self::TessellatedTriangle(t) => {
				map_panel_leaves(PanelGeometry::TessellatedTriangle(*t).flatten(panel_caps))
			}
			Self::Arc(g) => g.tiles(false),
			Self::SliceArc(g) => g.tiles(true),
		}
	}

	/// Expand tiles then compose under `parent` placement.
	pub fn placed_tiles(&self, parent: Placement) -> Vec<Placed<PartitionTile>> {
		self.placed_tiles_for_style(PartitionStyle::RoughStonework, parent)
	}

	pub fn placed_tiles_for_style(
		&self,
		style: PartitionStyle,
		parent: Placement,
	) -> Vec<Placed<PartitionTile>> {
		self.tiles_for_style(style)
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

fn map_panel_leaves(pieces: Vec<Placed<PanelGeometry>>) -> Vec<Placed<PartitionTile>> {
	pieces
		.into_iter()
		.filter_map(|p| {
			let tile = panel_to_tile(p.geom)?;
			Some(Placed { geom: tile, placement: adjust_panel_placement(tile, p.placement) })
		})
		.collect()
}

fn panel_to_tile(geom: PanelGeometry) -> Option<PartitionTile> {
	match geom {
		PanelGeometry::Rectangle(Rectangle) => Some(PartitionTile::Linear),
		PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
			Some(PartitionTile::RightTriangle { mirror })
		}
		_ => None,
	}
}

/// Ground-authored panel kits tip upright for wall use.
fn adjust_panel_placement(tile: PartitionTile, p: Placement) -> Placement {
	match tile {
		PartitionTile::Linear | PartitionTile::RightTriangle { .. } => with_wall_standup_pitch(p),
		_ => p,
	}
}

/// Discrete kit piece after tessellation (not a continuous authoring form).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionTile {
	Linear,
	LinearSubsegment,
	LinearSliceSubsegment,
	Arc180,
	Arc90,
	Arc15,
	SliceArc180,
	SliceArc90,
	SliceArc15,
	Joint,
	/// Unit right-triangle panel (edge caps / dual-triangle fill).
	RightTriangle {
		mirror: Option<MirrorAxis>,
	},
}

impl PartitionTile {
	/// High/mid/low mesh set when this tile has resolution variants.
	pub fn mesh_set(self) -> Option<PartitionMeshSet> {
		Some(match self {
			Self::Linear => PartitionMeshSet::new(LINEAR_HIGH, LINEAR_MID, LINEAR_LOW),
			Self::Arc180 => PartitionMeshSet::new(ARC_180_HIGH, ARC_180_MID, ARC_180_LOW),
			Self::Arc90 => PartitionMeshSet::new(ARC_90_HIGH, ARC_90_MID, ARC_90_LOW),
			Self::Arc15 => PartitionMeshSet::new(ARC_15_HIGH, ARC_15_MID, ARC_15_LOW),
			Self::SliceArc90 => PartitionMeshSet::new(SLICE_90_HIGH, SLICE_90_MID, SLICE_90_LOW),
			Self::SliceArc15 => PartitionMeshSet::new(SLICE_15_HIGH, SLICE_15_MID, SLICE_15_LOW),
			Self::Joint
			| Self::LinearSubsegment
			| Self::LinearSliceSubsegment
			| Self::SliceArc180
			| Self::RightTriangle { .. } => return None,
		})
	}
}

impl From<ArcKit> for PartitionTile {
	fn from(kit: ArcKit) -> Self {
		match kit {
			ArcKit::D180 => Self::Arc180,
			ArcKit::D90 => Self::Arc90,
			ArcKit::D15 => Self::Arc15,
		}
	}
}

pub(crate) fn slice_tile(kit: ArcKit) -> PartitionTile {
	match kit {
		ArcKit::D180 => PartitionTile::SliceArc180,
		ArcKit::D90 => PartitionTile::SliceArc90,
		ArcKit::D15 => PartitionTile::SliceArc15,
	}
}
