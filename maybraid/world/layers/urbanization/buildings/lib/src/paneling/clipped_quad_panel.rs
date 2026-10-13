//! One ruled quad with an optional closed clip → [`PanelComplex`].
//!
//! Diagonal \(a_0\)–\(b_1\) splits into two [`ClippedTessellatedTriangle`]s that share
//! the same world clip (each projects onto its own plane). Present via
//! [`Self::into_complex`].

use bevy_math::Vec3;
use building_components::panels::PanelStyle;

use crate::paneling::clipped_tessellated_triangle::ClippedTessellatedTriangle;
use crate::paneling::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use crate::paneling::quad_panel::QuadPanel;

/// Thin wrapper: clipped (or solid) ruled quad as a [`PanelComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedQuadPanel {
	pub style: PanelStyle,
	pub a0: PanelPoint,
	pub a1: PanelPoint,
	pub b0: PanelPoint,
	pub b1: PanelPoint,
	/// Closed clip polyline (world). Empty → solid quad.
	pub clip: Vec<Vec3>,
	complex: PanelComplex,
}

impl ClippedQuadPanel {
	/// Build eagerly. Empty `clip` → solid [`QuadPanel`] triangulation.
	pub fn new(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		let a0 = a0.into();
		let a1 = a1.into();
		let b0 = b0.into();
		let b1 = b1.into();
		let clip: Vec<Vec3> = clip.into_iter().map(Into::into).collect();
		let complex =
			build_complex(style, a0, a1, b0, b1, &clip, PanelComplexJointPolicy::default());
		Self { style, a0, a1, b0, b1, clip, complex }
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1, clip)
	}

	pub fn shepherds_thatch(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a0, a1, b0, b1, clip)
	}

	/// Solid ruled quad (no clip).
	pub fn solid(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		Self::new(style, a0, a1, b0, b1, std::iter::empty::<Vec3>())
	}

	pub fn with_clip(self, clip: impl IntoIterator<Item = impl Into<Vec3>>) -> Self {
		let policy = self.complex.joint_policy;
		Self::new(self.style, self.a0, self.a1, self.b0, self.b1, clip).with_joint_policy(policy)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.complex.set_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}
}

impl AsRef<PanelComplex> for ClippedQuadPanel {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<ClippedQuadPanel> for PanelComplex {
	fn from(value: ClippedQuadPanel) -> Self {
		value.into_complex()
	}
}

fn build_complex(
	style: PanelStyle,
	a0: PanelPoint,
	a1: PanelPoint,
	b0: PanelPoint,
	b1: PanelPoint,
	clip: &[Vec3],
	joint_policy: PanelComplexJointPolicy,
) -> PanelComplex {
	if clip.is_empty() {
		return QuadPanel::new(style, a0, a1, b0, b1)
			.with_joint_policy(joint_policy)
			.into_complex();
	}

	// Diagonal a0–b1: (a0,a1,b1) and (a0,b1,b0).
	let t0 = ClippedTessellatedTriangle::new(
		style,
		a0.position,
		a1.position,
		b1.position,
		clip.iter().copied(),
	)
	.with_joint_policy(joint_policy);
	let t1 = ClippedTessellatedTriangle::new(
		style,
		a0.position,
		b1.position,
		b0.position,
		clip.iter().copied(),
	)
	.with_joint_policy(joint_policy);

	let mut complex = t0.into_complex();
	complex.append_complex(t1.into_complex());
	complex
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::paneling::panel_plane::panel_plane_frame;
	use bevy_math::Vec3;

	fn ground_quad() -> (Vec3, Vec3, Vec3, Vec3) {
		(Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 2.0), Vec3::new(3.0, 0.0, 2.0))
	}

	fn interior_trap() -> [Vec3; 4] {
		[
			Vec3::new(0.8, 0.0, 0.5),
			Vec3::new(1.8, 0.0, 0.5),
			Vec3::new(1.6, 0.0, 1.2),
			Vec3::new(1.0, 0.0, 1.2),
		]
	}

	fn point_in_tri_strict(
		p: bevy_math::Vec2,
		a: bevy_math::Vec2,
		b: bevy_math::Vec2,
		c: bevy_math::Vec2,
	) -> bool {
		let cross = |o: bevy_math::Vec2, u: bevy_math::Vec2, v: bevy_math::Vec2| {
			(u.x - o.x) * (v.y - o.y) - (u.y - o.y) * (v.x - o.x)
		};
		let area = cross(a, b, c);
		if area.abs() < 1e-12 {
			return false;
		}
		let ab = cross(a, b, p);
		let bc = cross(b, c, p);
		let ca = cross(c, a, p);
		if area > 0.0 {
			ab > 1e-8 && bc > 1e-8 && ca > 1e-8
		} else {
			ab < -1e-8 && bc < -1e-8 && ca < -1e-8
		}
	}

	#[test]
	fn solid_has_two_tris() {
		let (a0, a1, b0, b1) = ground_quad();
		let q = ClippedQuadPanel::solid(PanelStyle::RoughStonework, a0, a1, b0, b1);
		assert_eq!(q.as_complex().triangles().len(), 2);
	}

	#[test]
	fn interior_clip_leaves_probe_empty() {
		let (a0, a1, b0, b1) = ground_quad();
		let clip = interior_trap();
		let q = ClippedQuadPanel::rough_stone(a0, a1, b0, b1, clip);
		assert!(q.as_complex().triangles().len() >= 3);
		let frame = panel_plane_frame(a0, a1, b0).unwrap();
		let probe = frame.project(Vec3::new(1.2, 0.0, 0.8));
		for tri in q.as_complex().triangles() {
			let pa = frame.project(q.as_complex().point(tri.a).unwrap().position);
			let pb = frame.project(q.as_complex().point(tri.b).unwrap().position);
			let pc = frame.project(q.as_complex().point(tri.c).unwrap().position);
			assert!(
				!point_in_tri_strict(probe, pa, pb, pc),
				"probe inside clip should not be covered"
			);
		}
	}

	#[test]
	fn oversized_clip_still_bites() {
		let (a0, a1, b0, b1) = ground_quad();
		let clip = [
			Vec3::new(-0.5, 0.0, 0.4),
			Vec3::new(3.5, 0.0, 0.4),
			Vec3::new(3.5, 0.0, 1.5),
			Vec3::new(-0.5, 0.0, 1.5),
		];
		let q = ClippedQuadPanel::rough_stone(a0, a1, b0, b1, clip);
		let n = q.as_complex().triangles().len();
		assert!(n >= 2, "expected bitten fill, got {n}");
		assert_ne!(n, 2, "oversized clip should not leave a solid two-tri quad");
	}

	/// Ground-flush door that crosses the a0–b1 diagonal used to drop one side of
	/// the lower triangle when `bite_polygon` kept only a single fill loop.
	#[test]
	fn ground_door_keeps_both_sides_of_opening() {
		let a0 = Vec3::new(0.0, 0.0, 0.0);
		let a1 = Vec3::new(0.0, 3.0, 0.0);
		let b0 = Vec3::new(4.0, 0.0, 0.0);
		let b1 = Vec3::new(4.0, 3.0, 0.0);
		// Centered door: width 1, height 2.1 (0.7 of face) — crosses diagonal y=(3/4)x.
		let clip = [
			Vec3::new(1.5, 0.0, 0.0),
			Vec3::new(2.5, 0.0, 0.0),
			Vec3::new(2.5, 2.1, 0.0),
			Vec3::new(1.5, 2.1, 0.0),
		];
		let q = ClippedQuadPanel::rough_stone(a0, a1, b0, b1, clip);
		let complex = q.as_complex();
		assert!(complex.triangles().len() >= 4, "expected multi-component fill");

		let covered = |p: Vec3| -> bool {
			for tri in complex.triangles() {
				let pa = complex.point(tri.a).unwrap().position;
				let pb = complex.point(tri.b).unwrap().position;
				let pc = complex.point(tri.c).unwrap().position;
				// In XY plane (z=0): use X/Y as 2D.
				let to2 = |v: Vec3| bevy_math::Vec2::new(v.x, v.y);
				if point_in_tri_strict(to2(p), to2(pa), to2(pb), to2(pc)) {
					return true;
				}
			}
			false
		};

		assert!(covered(Vec3::new(0.5, 0.15, 0.0)), "left of door near ground should remain");
		assert!(covered(Vec3::new(3.5, 0.15, 0.0)), "right of door near ground should remain");
		assert!(!covered(Vec3::new(2.0, 0.5, 0.0)), "door interior should be open");
		assert!(covered(Vec3::new(2.0, 2.6, 0.0)), "above door should remain");
	}
}
