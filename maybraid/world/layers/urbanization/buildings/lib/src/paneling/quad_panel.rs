//! Single-quad construction wrapper over [`PanelComplex`].
//!
//! Corners \((a_0, a_1, b_0, b_1)\) triangulate with diagonal \(a_0\)–\(b_1\).
//! Use [`Self::into_complex`] to continue editing (e.g. add more triangles).

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::partitions::PANEL_Y_HALF;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPoint, PanelPointId,
};

/// Re-export for call sites that imported thickness from this module.
pub use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;

/// Thin wrapper: one ruled quad as a [`PanelComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct QuadPanel(PanelComplex);

impl QuadPanel {
	pub fn new(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		let a0 = a0.into();
		let a1 = a1.into();
		let b0 = b0.into();
		let b1 = b1.into();
		let mut c = PanelComplex::new(style);
		let id0 = c.insert_point_thick(a0.position, a0.thickness);
		let id1 = c.insert_point_thick(a1.position, a1.thickness);
		let id2 = c.insert_point_thick(b0.position, b0.thickness);
		let id3 = c.insert_point_thick(b1.position, b1.thickness);
		c.add_quad(id0, id1, id2, id3);
		Self(c)
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1)
	}

	pub fn shepherds_thatch(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a0, a1, b0, b1)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.0 = self.0.with_joint_policy(joint_policy);
		self
	}

	pub fn into_complex(self) -> PanelComplex {
		self.0
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.0
	}

	/// Level slab whose **kit top** sits on `(a0, a1, b0, b1)`.
	///
	/// Rectangle / triangle panels are centered on local \(Y = 0\) with
	/// half-extent [`PANEL_Y_HALF`]. This shifts those corners down the plane
	/// normal so the kit top stays on the authored plane.
	pub fn slab(style: PanelStyle, a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3, thickness: f32) -> Self {
		let n = (a1 - a0).cross(b0 - a0).normalize_or_zero();
		let down = if n.y >= 0.0 { -n } else { n };
		let shift = down * PANEL_Y_HALF;
		let t = |p: Vec3| PanelPoint::new(p + shift, thickness);
		Self::new(style, t(a0), t(a1), t(b0), t(b1))
	}

	/// Authored corners \((a_0, a_1, b_0, b_1)\).
	pub fn corners(&self) -> [Vec3; 4] {
		[0, 1, 2, 3]
			.map(|i| self.0.point(PanelPointId(i)).map(|p| p.position).unwrap_or(Vec3::ZERO))
	}

	/// Kit thickness at \(a_0\) (uniform for [`Self::slab`]).
	pub fn thickness(&self) -> f32 {
		self.0
			.point(PanelPointId(0))
			.map(|p| p.thickness)
			.unwrap_or(DEFAULT_PANEL_THICKNESS)
	}
}

impl BuildingComponents for QuadPanel {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.0.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.0.joint_nodes_for_level(level)
	}
}

impl AsRef<PanelComplex> for QuadPanel {
	fn as_ref(&self) -> &PanelComplex {
		&self.0
	}
}

impl From<QuadPanel> for PanelComplex {
	fn from(value: QuadPanel) -> Self {
		value.into_complex()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::{EulerRot, Quat, Vec3};
	use building_components::joints::JOINT_KIT_XZ;
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;

	#[test]
	fn coplanar_emits_two_panels_and_no_joint() {
		let c = QuadPanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(2.0, 0.0, 2.0),
		)
		.into_complex();
		assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(), 2);
		let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
		assert!(kink < 1e-3, "expected near-coplanar, got {kink}");
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	fn folded_joint_aligns_y_with_diagonal_and_xz_with_thickness() {
		let thick = 0.25;
		let a0 = PanelPoint::new(Vec3::ZERO, thick);
		let a1 = PanelPoint::new(Vec3::new(1.0, 0.0, 0.0), thick);
		let b0 = PanelPoint::new(Vec3::new(0.0, 1.0, 0.0), thick);
		let b1 = PanelPoint::new(Vec3::new(0.0, 0.0, 1.0), thick);
		let c = QuadPanel::rough_stone(a0, a1, b0, b1).into_complex();
		let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
		assert!(
			(kink - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
			"expected ~90° fold, got {kink}"
		);
		let joints = c.joint_nodes();
		assert_eq!(joints.len(), 1);
		let p = &joints[0].placement;
		assert!((p.translation - a0.position).length() < 1e-4);
		let diag = (b1.position - a0.position).normalize();
		let rot = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, p.roll);
		let y_axis = rot * Vec3::Y;
		assert!(
			(y_axis - diag).length() < 1e-3 || (y_axis + diag).length() < 1e-3,
			"kit +Y should align with diagonal, got {y_axis:?} vs {diag:?}"
		);
		assert!((p.scale.y - (b1.position - a0.position).length()).abs() < 1e-4);
		let want_xz = thick / JOINT_KIT_XZ;
		assert!((p.scale.x - want_xz).abs() < 1e-4);
		assert!((p.scale.z - want_xz).abs() < 1e-4);
	}

	#[test]
	fn edge_thickness_averages_diagonal_endpoints() {
		let c = QuadPanel::rough_stone(
			PanelPoint::new(Vec3::ZERO, 0.2),
			PanelPoint::new(Vec3::new(1.0, 0.0, 0.0), 0.4),
			PanelPoint::new(Vec3::new(0.0, 1.0, 0.0), 0.6),
			PanelPoint::new(Vec3::new(0.0, 0.0, 1.0), 0.8),
		)
		.into_complex();
		// Shared diagonal a0–b1: avg(0.2, 0.8) = 0.5
		let e = c.shared_edges()[0];
		assert!((c.edge_thickness(e.a, e.b).unwrap() - 0.5).abs() < 1e-5);
	}

	#[test]
	fn never_policy_suppresses_joint() {
		let c = QuadPanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(0.0, 1.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
		)
		.with_joint_policy(PanelComplexJointPolicy::never())
		.into_complex();
		assert!(c.joint_nodes().is_empty());
	}

	#[test]
	fn into_complex_allows_extra_triangle() {
		let mut c = QuadPanel::rough_stone(Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z).into_complex();
		let p = c.insert_point(Vec3::new(2.0, 0.0, 0.0));
		let q = c.insert_point(Vec3::new(2.0, 1.0, 0.0));
		let r = c.insert_point(Vec3::new(2.0, 0.0, 1.0));
		c.add_triangle(p, q, r);
		assert_eq!(c.triangles().len(), 3);
	}
}
