//! Short-run polyline partition (single LOD parent) and tessellation into tiles.

use bevy_math::{Vec2, Vec3};

use crate::panels::yaw_along_xz;
use crate::partitions::geometry::joint::JointPartition;
use crate::partitions::geometry::linear::{
	fitted_tile_count, DEFAULT_TILE_WIDTH, PANEL_TO_WALL_PITCH,
};
use crate::partitions::geometry::PartitionTile;
use crate::placed::{Placed, Placement};

pub use crate::panels::{roll_along_slope, DEFAULT_MIN_JOINT_ANGLE};

/// Short-run polyline. Prefer splitting long paths in higher-order walling/buildings.
///
/// One [`crate::partitions::PartitionNode`] is a single LOD parent for all kits.
/// Tile translations follow the **3D path** (slope is in the points). Panels stay
/// **plumb**: yaw in plan + stand-up pitch only — no slope tip roll.
///
/// Each edge is subdivided by **horizontal** length \(L_{xz}\) with [`Self::tile_width`]:
/// \(n = \mathrm{round}(L_{xz}/\texttt{tile\_width})\) tiles stretch to width \(L_{xz}/n\).
#[derive(Debug, Clone, PartialEq)]
pub struct PolylinePartition {
	pub points: Vec<Vec3>,
	/// Suggested full tile width along each edge; fitted so \(n\) tiles span exactly.
	pub tile_width: f32,
	/// Omit joint kits when both plan and slope kink angles are below this (radians).
	pub min_joint_angle: f32,
	/// Roll of the segment that ends at `points[0]` but is not part of this polyline.
	/// When set, evaluate a start joint at `points[0]` against the first edge.
	pub incoming_slope: Option<f32>,
	/// Storey height scale on kit \(Z\) (after stand-up pitch).
	pub wall_height: f32,
	/// Thickness scale on kit \(Y\) (after stand-up pitch).
	pub thick_scale: f32,
}

impl Default for PolylinePartition {
	fn default() -> Self {
		Self {
			points: Vec::new(),
			tile_width: DEFAULT_TILE_WIDTH,
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			incoming_slope: None,
			wall_height: 1.0,
			thick_scale: 1.0,
		}
	}
}

impl PolylinePartition {
	pub fn new(points: impl Into<Vec<Vec3>>) -> Self {
		Self {
			points: points.into(),
			tile_width: DEFAULT_TILE_WIDTH,
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			incoming_slope: None,
			wall_height: 1.0,
			thick_scale: 1.0,
		}
	}

	pub fn with_tile_width(mut self, tile_width: f32) -> Self {
		self.tile_width = tile_width.max(1e-4);
		self
	}

	pub fn with_min_joint_angle(mut self, min_joint_angle: f32) -> Self {
		self.min_joint_angle = min_joint_angle.max(0.0);
		self
	}

	pub fn with_incoming_slope(mut self, incoming_slope: f32) -> Self {
		self.incoming_slope = Some(incoming_slope);
		self
	}

	/// Kit \(Z\) height and \(Y\) thickness scales for ground-authored rectangle panels.
	pub fn with_wall_scale(mut self, wall_height: f32, thick_scale: f32) -> Self {
		self.wall_height = wall_height.max(1e-4);
		self.thick_scale = thick_scale.max(1e-4);
		self
	}

	/// Expand into posed linear + joint tiles (identity parent).
	pub fn tiles(&self) -> Vec<Placed<PartitionTile>> {
		let points = &self.points;
		if points.len() < 2 {
			return vec![Placed::with_placement(
				PartitionTile::Linear,
				Placement::at_origin().with_pitch(PANEL_TO_WALL_PITCH).with_scale(Vec3::new(
					1.0,
					self.thick_scale,
					self.wall_height,
				)),
			)];
		}

		let min_joint = self.min_joint_angle.max(0.0);
		let tile_width = self.tile_width.max(1e-4);
		let height = self.wall_height.max(1e-4);
		let thick = self.thick_scale.max(1e-4);
		let mut out = Vec::new();
		let n_edges = points.len() - 1;

		for i in 0..n_edges {
			let a = points[i];
			let b = points[i + 1];
			let delta = b - a;
			let horiz_len = (delta.x * delta.x + delta.z * delta.z).sqrt().max(1e-4);
			let yaw = yaw_along_xz(delta.x, delta.z);
			let n = fitted_tile_count(horiz_len, tile_width);
			let width = horiz_len / n as f32;
			for j in 0..n {
				// Path \(Y\) carries slope; panel stays plumb (yaw + stand-up only).
				let start = a + delta * (j as f32 / n as f32);
				out.push(Placed {
					geom: PartitionTile::Linear,
					placement: Placement::new(start, yaw)
						.with_pitch(PANEL_TO_WALL_PITCH)
						.with_scale(Vec3::new(width, thick, height)),
				});
			}
		}

		if let Some(roll_in) = self.incoming_slope {
			let a = points[0];
			let b = points[1];
			let dout = b - a;
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let droll = (roll_out - roll_in).abs();
			if droll >= min_joint {
				out.push(JointPartition::placed_at(a, yaw_out, yaw_out, roll_in, roll_out, height));
			}
		}

		for i in 1..points.len() - 1 {
			let prev = points[i - 1];
			let cur = points[i];
			let next = points[i + 1];
			let din = cur - prev;
			let dout = next - cur;
			let yaw_in = yaw_along_xz(din.x, din.z);
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let roll_in = roll_along_slope(din.x, din.y, din.z);
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let dyaw = wrap_pi(yaw_out - yaw_in).abs();
			let droll = (roll_out - roll_in).abs();
			let kink = dyaw.max(droll);
			if kink < min_joint {
				continue;
			}
			out.push(JointPartition::placed_at(cur, yaw_in, yaw_out, roll_in, roll_out, height));
		}

		out
	}
}

/// Convert legacy 2D polyline points (XZ) into 3D with \(Y = 0\).
pub fn polyline_from_xz(points: impl IntoIterator<Item = Vec2>) -> PolylinePartition {
	PolylinePartition::new(points.into_iter().map(|p| Vec3::new(p.x, 0.0, p.y)).collect::<Vec<_>>())
}

pub(crate) fn wrap_pi(a: f32) -> f32 {
	crate::panels::wrap_pi(a)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::PartitionGeometry;

	#[test]
	fn straight_polyline_has_no_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 0.0, 0.0),
		]);
		let pieces = g.tiles();
		assert_eq!(
			pieces.iter().filter(|p| p.geom == PartitionTile::Linear).count(),
			4 // two edges × round(2/1) tiles
		);
		assert!(!pieces.iter().any(|p| p.geom == PartitionTile::Joint));
		Ok(())
	}

	#[test]
	fn edge_tile_width_subdivides_and_fits() -> anyhow::Result<()> {
		let g = PartitionGeometry::Polyline(
			PolylinePartition::new([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.4, 0.0, 0.0)])
				.with_tile_width(1.0),
		);
		let linears: Vec<_> =
			g.tiles().into_iter().filter(|p| p.geom == PartitionTile::Linear).collect();
		// round(2.4/1)=2 tiles of width 1.2, origin-anchored at tile starts
		assert_eq!(linears.len(), 2);
		assert!((linears[0].scale().x - 1.2).abs() < 1e-4);
		assert!((linears[0].translation().x).abs() < 1e-4);
		assert!((linears[1].translation().x - 1.2).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn l_shape_emits_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
		]);
		assert_eq!(g.tiles().iter().filter(|p| p.geom == PartitionTile::Joint).count(), 1);
		Ok(())
	}

	#[test]
	fn small_plan_kink_omits_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::Polyline(
			PolylinePartition::new([
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.1),
			])
			.with_min_joint_angle(DEFAULT_MIN_JOINT_ANGLE),
		);
		assert!(!g.tiles().iter().any(|p| p.geom == PartitionTile::Joint));
		Ok(())
	}

	#[test]
	fn slope_kink_emits_joint_and_grows_radius() -> anyhow::Result<()> {
		let flat = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
		]);
		let sloped = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 2.0, 0.0),
		]);
		let flat_j = flat
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Joint)
			.ok_or_else(|| anyhow::anyhow!("flat joint"))?;
		let slope_j = sloped
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Joint)
			.ok_or_else(|| anyhow::anyhow!("slope joint"))?;
		assert!(
			slope_j.placement.scale.x > flat_j.placement.scale.x + 1e-3,
			"slope kink should grow joint radius"
		);
		// Plumb: slope sizes the joint only; no tip roll yet.
		assert!(slope_j.placement.roll.abs() < 1e-6);
		assert!(flat_j.placement.pitch.abs() < 1e-6);
		assert!((flat_j.placement.scale.y - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn joint_uses_wall_height_without_panel_pitch() -> anyhow::Result<()> {
		let g = PartitionGeometry::Polyline(
			PolylinePartition::new([
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 2.0),
			])
			.with_wall_scale(3.0, 0.75),
		);
		let joint = g
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Joint)
			.ok_or_else(|| anyhow::anyhow!("missing joint"))?;
		assert!(joint.placement.pitch.abs() < 1e-6);
		assert!((joint.placement.scale.y - 3.0).abs() < 1e-4);
		let linear = g
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Linear)
			.ok_or_else(|| anyhow::anyhow!("missing linear"))?;
		assert!((linear.placement.pitch - PANEL_TO_WALL_PITCH).abs() < 1e-4);
		assert!((linear.placement.scale.z - 3.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn sloping_edge_stays_plumb_and_follows_path_y() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)]);
		let linears: Vec<_> =
			g.tiles().into_iter().filter(|p| p.geom == PartitionTile::Linear).collect();
		assert_eq!(linears.len(), 2); // horiz 2 / tile_width 1
		for linear in &linears {
			assert!(linear.placement.roll.abs() < 1e-6);
			assert!((linear.placement.pitch - PANEL_TO_WALL_PITCH).abs() < 1e-4);
			assert!((linear.placement.scale.x - 1.0).abs() < 1e-4);
			let kit_x = linear.placement.rotation() * Vec3::X;
			assert!(kit_x.y.abs() < 1e-4, "plumb panel kit +X must stay horizontal, got {kit_x:?}");
		}
		assert!(linears[0].placement.translation.abs().max_element() < 1e-3);
		// Second tile start is halfway along the 3D edge (path Y carries slope).
		assert!((linears[1].placement.translation.y - 0.5).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn incoming_slope_emits_start_joint() -> anyhow::Result<()> {
		let flat_in = PartitionGeometry::Polyline(
			PolylinePartition::new([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)])
				.with_incoming_slope(0.0),
		);
		assert_eq!(flat_in.tiles().iter().filter(|p| p.geom == PartitionTile::Joint).count(), 1);
		let matched = PartitionGeometry::Polyline(
			PolylinePartition::new([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)])
				.with_incoming_slope(roll_along_slope(2.0, 1.0, 0.0)),
		);
		assert!(!matched.tiles().iter().any(|p| p.geom == PartitionTile::Joint));
		Ok(())
	}
}
