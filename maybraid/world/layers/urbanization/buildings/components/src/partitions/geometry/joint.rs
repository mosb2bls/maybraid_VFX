//! Circular joint tile for polyline wall kit expansion.

use bevy_math::Vec3;

use crate::joints::joint_xz_scale;
use crate::panels::wrap_pi;
use crate::partitions::geometry::PartitionTile;
use crate::placed::{Placed, Placement};

pub use crate::joints::{
	JointLod, JOINT_BASE_RADIUS, JOINT_HIGH_FACTOR, JOINT_KIT_HALF, JOINT_MEDIUM_FACTOR,
	JOINT_RADIUS_PER_SLOPE_RAD,
};

/// Circular / post joint between upright linear partition segments.
///
/// Authored vertically: \(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\). No stand-up pitch
/// and no slope tip — stay plumb; yaw bisects the plan turn; \(X/Z\) scale grows with
/// the vertical kink between abutting segments.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JointPartition;

impl JointPartition {
	/// Placement for a joint at `cur` bridging inbound/outbound plan (and slope) angles.
	///
	/// `roll_in` / `roll_out` size the joint only; they are not applied as rotation.
	/// `wall_height` scales kit \(Y\) so the joint spans the storey (kit \(Y \in [0, 1]\)).
	pub fn placed_at(
		cur: Vec3,
		yaw_in: f32,
		yaw_out: f32,
		roll_in: f32,
		roll_out: f32,
		wall_height: f32,
	) -> Placed<PartitionTile> {
		let droll = (roll_out - roll_in).abs();
		let xz = joint_xz_scale(droll);
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		Placed {
			geom: PartitionTile::Joint,
			placement: Placement::new(cur, yaw).with_scale(Vec3::new(
				xz,
				wall_height.max(1e-4),
				xz,
			)),
		}
	}
}
