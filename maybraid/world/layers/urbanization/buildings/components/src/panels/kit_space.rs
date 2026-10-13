//! Placement remaps between lower-left panel space and domain kit conventions.

use bevy_math::{Vec2, Vec3};
use scene_ref::MirrorAxis;

use crate::placed::Placement;

/// Kit half-thickness in \(Y\) (matches [`crate::partitions::PANEL_Y_HALF`]).
const KIT_Y_HALF: f32 = 0.2;

/// Unit rectangle / partition kit AABB: \(X \in [0, 1]\), \(Y \in [\pm 0.2]\), \(Z \in [-1, 0]\).
pub const PANEL_KIT_MIN: Vec3 = Vec3::new(0.0, -KIT_Y_HALF, -1.0);
/// See [`PANEL_KIT_MIN`].
pub const PANEL_KIT_MAX: Vec3 = Vec3::new(1.0, KIT_Y_HALF, 0.0);

/// Pitch that tips kit \(+Z\) (authored height edge) to world \(+Y\).
///
/// Matches [`crate::partitions::PANEL_TO_WALL_PITCH`].
pub const WALL_STANDUP_PITCH: f32 = std::f32::consts::FRAC_PI_2;

/// Convert panel-space lower-left rectangle placement (\(X,Z \in [0, 1]\)) to a
/// centered floor kit (\(X,Z \in [-1, 1]\)).
///
/// Panel scale is full edge length; floor scale is half-extent (\(L/2\)).
pub fn to_centered_rect_placement(p: Placement) -> Placement {
	let s = p.scale;
	// Unit-square panel: center at local \((0.5, 0, 0.5)\) before scale.
	let local_mid = Vec3::new(s.x * 0.5, 0.0, s.z * 0.5);
	Placement {
		translation: p.translation + p.rotation() * local_mid,
		yaw: p.yaw,
		pitch: p.pitch,
		roll: p.roll,
		scale: Vec3::new((s.x * 0.5).max(1e-4), s.y, (s.z * 0.5).max(1e-4)),
	}
}

/// Apply wall stand-up pitch to a ground-authored panel placement.
///
/// Keeps scale as \((\texttt{length}, \texttt{thick}, \texttt{height})\) on \((X,Y,Z)\).
pub fn with_wall_standup_pitch(p: Placement) -> Placement {
	Placement {
		translation: p.translation,
		yaw: p.yaw,
		pitch: p.pitch + WALL_STANDUP_PITCH,
		roll: p.roll,
		scale: p.scale,
	}
}

/// Local prism vertices for a unit right-triangle panel (\(X \in [0,1]\), \(Z \in [-1,0]\)).
///
/// Right angle at the origin; far corners \((1,0,0)\) and \((0,0,-1)\). `mirror` is the
/// same axis flip as the GLB (`SceneRef::with_mirror`).
pub fn right_triangle_kit_hull(scale: Vec3, mirror: Option<MirrorAxis>) -> Vec<Vec3> {
	let flip = match mirror {
		None => Vec3::ONE,
		Some(MirrorAxis::X) => Vec3::new(-1.0, 1.0, 1.0),
		Some(MirrorAxis::Y) => Vec3::new(1.0, -1.0, 1.0),
		Some(MirrorAxis::Z) => Vec3::new(1.0, 1.0, -1.0),
	};
	let s = scale * flip;
	let y = KIT_Y_HALF;
	[
		Vec3::new(0.0, -y, 0.0),
		Vec3::new(1.0, -y, 0.0),
		Vec3::new(0.0, -y, -1.0),
		Vec3::new(0.0, y, 0.0),
		Vec3::new(1.0, y, 0.0),
		Vec3::new(0.0, y, -1.0),
	]
	.into_iter()
	.map(|p| p * s)
	.collect()
}

/// Local prism vertices for a tessellated panel triangle in panel \(XZ\).
///
/// Corners are panel-space \((X, Z)\). Extrude \(\pm\) [`KIT_Y_HALF`] along kit \(Y\),
/// then apply `scale` (parent tessellated nodes use [`Vec3::ONE`]; triangle size
/// lives in the corners).
pub fn tessellated_triangle_kit_hull(a: Vec2, b: Vec2, c: Vec2, scale: Vec3) -> Vec<Vec3> {
	let y = KIT_Y_HALF;
	[a, b, c]
		.into_iter()
		.flat_map(|p| [Vec3::new(p.x, -y, p.y) * scale, Vec3::new(p.x, y, p.y) * scale])
		.collect()
}

/// Inscribed-square half-extent as a fraction of the unit circle (kit README).
pub const INSCRIBED_SQUARE_HALF: f32 = 0.7;

/// Southern circle−square cap in centered floor kit space.
///
/// Unit circle \(X^2+Z^2\le 1\), square \(X,Z\in[\pm 0.7]\). The remaining \(+Z\)
/// segment; four yaws fill a circular ring. Chord + arc samples, extruded
/// \(\pm\) [`KIT_Y_HALF`], then scaled.
pub fn circle_inscribed_square_kit_hull(scale: Vec3) -> Vec<Vec3> {
	const ARC: u32 = 6;
	let half = INSCRIBED_SQUARE_HALF.clamp(0.0, 0.999).acos();
	let y = KIT_Y_HALF;
	(0..=ARC)
		.flat_map(|i| {
			let t = -half + (2.0 * half) * (i as f32 / ARC as f32);
			let xz = Vec2::new(t.sin(), t.cos());
			[Vec3::new(xz.x, -y, xz.y) * scale, Vec3::new(xz.x, y, xz.y) * scale]
		})
		.collect()
}

/// Eight local corners of a scaled rectangle kit (origin at the eave / \(X{=}0,Z{=}0\)).
pub fn rectangle_kit_hull(scale: Vec3) -> Vec<Vec3> {
	let min = PANEL_KIT_MIN * scale;
	let max = PANEL_KIT_MAX * scale;
	vec![
		Vec3::new(min.x, min.y, min.z),
		Vec3::new(max.x, min.y, min.z),
		Vec3::new(min.x, max.y, min.z),
		Vec3::new(max.x, max.y, min.z),
		Vec3::new(min.x, min.y, max.z),
		Vec3::new(max.x, min.y, max.z),
		Vec3::new(min.x, max.y, max.z),
		Vec3::new(max.x, max.y, max.z),
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tessellated_hull_keeps_the_three_corners() {
		let a = Vec2::ZERO;
		let b = Vec2::new(2.0, 0.0);
		let c = Vec2::new(0.0, 3.0);
		let pts = tessellated_triangle_kit_hull(a, b, c, Vec3::ONE);
		assert_eq!(pts.len(), 6);
		for corner in [a, b, c] {
			assert!(
				pts.iter().any(|p| (Vec2::new(p.x, p.z) - corner).length() < 1e-4 && p.y > 0.0),
				"missing top corner {corner:?} in {pts:?}"
			);
			assert!(
				pts.iter().any(|p| (Vec2::new(p.x, p.z) - corner).length() < 1e-4 && p.y < 0.0),
				"missing bottom corner {corner:?} in {pts:?}"
			);
		}
	}

	#[test]
	fn inscribed_cap_hull_is_the_plus_z_segment() {
		let pts = circle_inscribed_square_kit_hull(Vec3::ONE);
		assert_eq!(pts.len(), 14);
		assert!(pts.iter().any(|p| (p.z - 1.0).abs() < 1e-4 && p.x.abs() < 1e-4));
		assert!(pts.iter().any(|p| (p.z - INSCRIBED_SQUARE_HALF).abs() < 1e-3));
		assert!(pts.iter().all(|p| p.z >= INSCRIBED_SQUARE_HALF - 1e-3));
	}

	#[test]
	fn rectangle_hull_matches_neg_z_kit_aabb() {
		let pts = rectangle_kit_hull(Vec3::new(2.0, 0.75, 1.0));
		assert_eq!(pts.len(), 8);
		assert!(pts
			.iter()
			.any(|p| (*p - Vec3::new(0.0, -KIT_Y_HALF * 0.75, -1.0)).length() < 1e-4));
		assert!(pts
			.iter()
			.any(|p| (*p - Vec3::new(2.0, KIT_Y_HALF * 0.75, 0.0)).length() < 1e-4));
	}
}
