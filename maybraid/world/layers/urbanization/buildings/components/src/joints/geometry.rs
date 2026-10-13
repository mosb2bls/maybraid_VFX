//! Continuous joint geometry and placement helpers.
//!
//! Kit space: \(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\) — origin at the base of the post;
//! \(+Y\) is the post axis.

use bevy_math::{EulerRot, Mat3, Quat, Vec3};

use crate::panels::wrap_pi;
use crate::placed::{Placed, Placement};

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
pub const JOINT_KIT_HALF: f32 = 0.5;
/// Full kit span in \(X\) or \(Z\) before scale (\(2 \times\) [`JOINT_KIT_HALF`]).
pub const JOINT_KIT_XZ: f32 = 2.0 * JOINT_KIT_HALF;
/// Base world radius when the kink is purely planar.
pub const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of kink (plan, slope, or dihedral).
pub const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;

/// Continuous joint form. Authored vertically: \(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JointGeometry {
	/// Circular post / crease filler.
	Post(JointPost),
}

impl Default for JointGeometry {
	fn default() -> Self {
		Self::post()
	}
}

impl JointGeometry {
	pub fn post() -> Self {
		Self::Post(JointPost)
	}
}

/// Alias kept for migration; prefer [`JointGeometry`].
pub type Joint = JointGeometry;

/// Unit circular joint post.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct JointPost;

/// World \(X/Z\) scale from a kink angle (radians).
pub fn joint_xz_scale(kink_rad: f32) -> f32 {
	let radius = JOINT_BASE_RADIUS + JOINT_RADIUS_PER_SLOPE_RAD * kink_rad.abs();
	(radius / JOINT_KIT_HALF).max(1e-4)
}

impl JointPost {
	/// Placement at `cur` bridging inbound/outbound plan and slope angles.
	///
	/// `height` scales kit \(Y\) (kit \(Y \in [0, 1]\)).
	pub fn placed_at(
		cur: Vec3,
		yaw_in: f32,
		yaw_out: f32,
		roll_in: f32,
		roll_out: f32,
		height: f32,
	) -> Placed<JointGeometry> {
		let dyaw = wrap_pi(yaw_out - yaw_in).abs();
		let droll = (roll_out - roll_in).abs();
		let kink = dyaw.max(droll);
		let xz = joint_xz_scale(kink);
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		Placed {
			geom: JointGeometry::post(),
			placement: Placement::new(cur, yaw).with_scale(Vec3::new(xz, height.max(1e-4), xz)),
		}
	}

	/// Placement along a crease: kit \(+Y\) from `start` toward `end`, \(X/Z\) sized to
	/// `thickness` (world diameter of the post).
	///
	/// `radial_hint` orients kit \(+X\) in the plane ⊥ edge (projected); when near-parallel
	/// to the edge a stable perpendicular is chosen.
	///
	/// Returns [`None`] when `start`≈`end` or `thickness` is non-positive.
	pub fn placed_along_crease(
		start: Vec3,
		end: Vec3,
		thickness: f32,
		radial_hint: Vec3,
	) -> Option<Placement> {
		let edge = end - start;
		let len = edge.length();
		if len < 1e-8 || thickness <= 1e-8 {
			return None;
		}
		let ey = edge / len;
		let mut ex = radial_hint - ey * radial_hint.dot(ey);
		if ex.length_squared() < 1e-10 {
			// Pick a stable axis not parallel to `ey`.
			let helper = if ey.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
			ex = helper.cross(ey);
		}
		let ex = ex.normalize();
		// Right-handed kit frame: columns `(ex, ey, ez)` with `ez = ex × ey`.
		let ez = ex.cross(ey);
		let rotation = Quat::from_mat3(&Mat3::from_cols(ex, ey, ez));
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		// Kit \(X,Z\) span is 1 before scale → `thickness` is the world diameter.
		let xz = (thickness / JOINT_KIT_XZ).max(1e-4);
		Some(Placement { translation: start, yaw, pitch, roll, scale: Vec3::new(xz, len, xz) })
	}
}

#[cfg(test)]
mod crease_placement_tests {
	use super::*;

	#[test]
	fn crease_y_aligns_with_edge_through_yxz_euler() {
		let start = Vec3::ZERO;
		let end = Vec3::new(0.0, 0.0, 1.0);
		let radial = Vec3::new(-1.0, -1.0, 0.0);
		let p = JointPost::placed_along_crease(start, end, 0.4, radial).expect("placement");
		let y = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, p.roll) * Vec3::Y;
		let edge = (end - start).normalize();
		assert!(
			(y - edge).length() < 1e-3 || (y + edge).length() < 1e-3,
			"kit +Y should align with edge, got {y:?} vs {edge:?}"
		);
		assert!((p.scale.y - 1.0).abs() < 1e-4);
		assert!((p.scale.x - 0.4 / JOINT_KIT_XZ).abs() < 1e-4);
	}
}
