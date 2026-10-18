//! Full-height linear partition geometry and LOD policy.
//!
//! Rectangle panels are authored on the ground (\(X,Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\)).
//! Wall use scales \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X, Y, Z)\),
//! then pitches \(\pi/2\) about \(+X\) so kit \(+Z\) stands up as world height (kit \(Y\)
//! becomes wall thickness). Segment anchors are the **lower-left** (kit origin).
//!
//! Stand-up pitch only — polyline path \(Y\) carries slope; panels stay plumb.

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use bevy_math::{Quat, Vec3};
use lod::scene::LodSceneLevel;
use scene_ref::MirrorAxis;

use crate::partitions::geometry::PartitionTile;
use crate::partitions::host::{posed_mesh_tier, posed_mirrored_mesh_tier};
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::probe::{PartitionLodBand, PartitionLodProbe};
use crate::placed::{Placed, Placement};

pub use crate::panels::fitted_tile_count;

/// `distance / max_extent` out to this → High.
pub const LINEAR_HIGH_FACTOR: f32 = 5.0;
/// Out to this → Medium.
pub const LINEAR_MEDIUM_FACTOR: f32 = 20.0;
/// Out to this → Low; else UltraLow.
pub const LINEAR_LOW_FACTOR: f32 = 500.0;

/// Kit half-extent in \(Y\) for the ground-authored rectangle panel (\([-0.2, 0.2]\)).
pub const PANEL_Y_HALF: f32 = 0.2;

/// Default linear thickness scale (\(0.15\) world / [`PANEL_Y_HALF`]).
pub const DEFAULT_THICK: f32 = 0.15 / PANEL_Y_HALF;

/// Suggested full tile width along local \(X\) (matches unscaled kit \(X \in [0, 1]\)).
///
/// Formerly \(2\) when the kit was \(X \in [-1, 1]\).
pub const DEFAULT_TILE_WIDTH: f32 = 1.0;

/// Pitch that tips kit \(+Z\) (authored height edge) to world \(+Y\).
///
/// With scale \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X,Y,Z)\):
/// \(X\) stays along the wall, \(Y\) becomes thickness, \(Z\) becomes storey height.
pub const PANEL_TO_WALL_PITCH: f32 = std::f32::consts::FRAC_PI_2;

fn panel_wall_pose(
	origin: Vec3,
	yaw: f32,
	length: f32,
	height: f32,
	thick_scale: f32,
) -> Placement {
	Placement::new(origin, yaw)
		.with_pitch(PANEL_TO_WALL_PITCH)
		.with_scale(Vec3::new(length.max(1e-4), thick_scale.max(1e-4), height.max(1e-4)))
}

/// Wall placement from the old centered half-extent convention.
///
/// `half_length` is what callers used as `scale.x` when the kit was \(X \in [-1, 1]\)
/// (world span \(2 \times \texttt{half\_length}\)). The ground panel is \(X \in [0, 1]\), so
/// this converts to full span on \(X\), anchors at the lower-left, and applies
/// [`PANEL_TO_WALL_PITCH`].
pub fn wall_placement_from_centered(
	mid_base: Vec3,
	yaw: f32,
	half_length: f32,
	height: f32,
	thick_scale: f32,
) -> Placement {
	let half = half_length.max(0.0);
	let length = (half * 2.0).max(1e-4);
	let origin = mid_base + Quat::from_rotation_y(yaw) * Vec3::new(-half, 0.0, 0.0);
	panel_wall_pose(origin, yaw, length, height, thick_scale)
}

/// Wall placement anchored at the lower-left / span-start corner.
///
/// `length` is the full world span along local \(+X\) (kit \(X \in [0, 1]\)).
pub fn wall_placement(
	origin: Vec3,
	yaw: f32,
	length: f32,
	height: f32,
	thick_scale: f32,
) -> Placement {
	panel_wall_pose(origin, yaw, length, height, thick_scale)
}

/// Unit linear partition from the ground rectangle panel
/// (\(X,Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\)).
///
/// Tessellation emits **unpitched** tiles: the stand-up pitch lives on the parent
/// [`wall_placement`] / spanning parent so it is applied once. Parent scale is
/// \((\texttt{length\_or\_1}, \texttt{thick\_scale}, \texttt{height})\). When
/// [`Self::length`] is `Some`, child tiles set \(X\) to the fitted **full** tile
/// width (not half — kit \(X\) is \([0, 1]\), not \([-1, 1]\)) and parent leaves
/// \(X\) at \(1\).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearPartition {
	/// World-space span along local \(X\). `None` → single unit kit.
	pub length: Option<f32>,
	/// Suggested tile full-width along \(X\); fitted when [`Self::length`] is set.
	pub tile_width: f32,
}

impl Default for LinearPartition {
	fn default() -> Self {
		Self { length: None, tile_width: DEFAULT_TILE_WIDTH }
	}
}

impl LinearPartition {
	pub fn new() -> Self {
		Self::default()
	}

	/// Continuous span tessellated with a suggested tile width.
	pub fn spanning(length: f32, tile_width: f32) -> Self {
		Self { length: Some(length.max(0.0)), tile_width: tile_width.max(1e-4) }
	}

	pub fn with_length(mut self, length: f32) -> Self {
		self.length = Some(length.max(0.0));
		self
	}

	pub fn with_tile_width(mut self, tile_width: f32) -> Self {
		self.tile_width = tile_width.max(1e-4);
		self
	}

	/// Expand into posed linear tiles (identity parent).
	///
	/// Stand-up pitch is **not** applied here — the parent placement must include
	/// [`PANEL_TO_WALL_PITCH`] (via [`wall_placement`] or an equivalent spanning parent).
	pub fn tiles(self) -> Vec<Placed<PartitionTile>> {
		let Some(length) = self.length.filter(|l| *l > 1e-6) else {
			return vec![Placed::at_origin(PartitionTile::Linear)];
		};
		let n = fitted_tile_count(length, self.tile_width);
		let width = length / n as f32;
		let mut out = Vec::with_capacity(n as usize);
		for i in 0..n {
			// Lower-left of each tile along local X (kit origin).
			let x = -length * 0.5 + i as f32 * width;
			out.push(Placed {
				geom: PartitionTile::Linear,
				placement: Placement::new(Vec3::new(x, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(width, 1.0, 1.0)),
			});
		}
		out
	}
}

/// LOD banding / posed mesh helpers for linear (and polyline-parent) partitions.
pub struct LinearLod;

impl LinearLod {
	pub fn band_from_distance_factor(factor: f32) -> PartitionLodBand {
		PartitionLodBand::from_distance_factor(factor)
	}

	pub fn level_for_placement(placement: &Placement, viewer: &Transform) -> LodSceneLevel {
		PartitionLodProbe::from_placement(placement).level_for(viewer)
	}

	pub fn posed_tier(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		posed_mesh_tier(meshes, transform, level)
	}

	/// [`Self::posed_tier`] with optional [`scene_ref::SceneRef`] axis mirroring.
	pub fn posed_mirrored_tier(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
		mirror: Option<MirrorAxis>,
	) -> impl Scene + 'static {
		posed_mirrored_mesh_tier(meshes, transform, level, mirror)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unit_linear_is_single_tile() -> anyhow::Result<()> {
		assert_eq!(LinearPartition::default().tiles().len(), 1);
		assert!(LinearPartition::default().tiles()[0].pitch().abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn spanning_fits_tiles_to_length() -> anyhow::Result<()> {
		let tiles = LinearPartition::spanning(3.0, 1.0).tiles();
		assert_eq!(tiles.len(), 3);
		// Full width on X (kit [0,1]), lower-left starts.
		assert!((tiles[0].scale().x - 1.0).abs() < 1e-4);
		assert!((tiles[0].translation().x - (-1.5)).abs() < 1e-4);
		assert!((tiles[1].translation().x - (-0.5)).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn tile_width_suggestion_rounds_and_stretches() -> anyhow::Result<()> {
		let tiles = LinearPartition::spanning(2.4, 1.0).tiles();
		assert_eq!(tiles.len(), 2);
		assert!((tiles[0].scale().x - 1.2).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn wall_placement_from_centered_doubles_old_half_extent() -> anyhow::Result<()> {
		let p =
			wall_placement_from_centered(Vec3::new(2.0, 0.0, 0.0), 0.0, 2.0, 3.0, DEFAULT_THICK);
		assert!((p.translation.x - 0.0).abs() < 1e-4);
		assert!((p.scale.x - 4.0).abs() < 1e-4);
		assert!((p.scale.y - DEFAULT_THICK).abs() < 1e-4);
		assert!((p.scale.z - 3.0).abs() < 1e-4);
		assert!((p.pitch - PANEL_TO_WALL_PITCH).abs() < 1e-4);
		Ok(())
	}
}
