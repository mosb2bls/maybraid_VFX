use bevy_math::{EulerRot, Quat, Vec3};
use building_components::joints::JOINT_KIT_XZ;
use building_components::BuildingComponents;
use lod::scene::LodSceneLevel;

use super::adjacency::canonical_edge;
use super::{shared_edges, PanelComplex, PanelComplexJointPolicy, PanelPointId};

fn folded_quad() -> PanelComplex {
	let mut c = PanelComplex::rough_stone();
	let a0 = c.insert_point_thick(Vec3::ZERO, 0.25);
	let a1 = c.insert_point_thick(Vec3::new(1.0, 0.0, 0.0), 0.25);
	let b0 = c.insert_point_thick(Vec3::new(0.0, 1.0, 0.0), 0.25);
	let b1 = c.insert_point_thick(Vec3::new(0.0, 0.0, 1.0), 0.25);
	c.add_triangle(a0, a1, b1).add_triangle(a0, b1, b0);
	c
}

#[test]
fn builder_quad_emits_two_panels_and_one_shared_edge() {
	let c = folded_quad();
	assert_eq!(c.points().count(), 4);
	assert_eq!(c.triangles().len(), 2);
	assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(), 2);
	let shared = c.shared_edges();
	assert_eq!(shared.len(), 1);
	let (u, v) = shared[0].endpoints();
	assert_eq!(canonical_edge(u, v), canonical_edge(PanelPointId(0), PanelPointId(3)));
}

#[test]
fn folded_joint_aligns_y_and_uses_endpoint_avg_thickness() {
	let c = folded_quad();
	let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
	assert!((kink - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "expected ~90° fold, got {kink}");
	let joints = c.joint_nodes();
	assert_eq!(joints.len(), 1);
	let p = &joints[0].placement;
	assert!((p.translation - Vec3::ZERO).length() < 1e-4);
	let diag = Vec3::new(0.0, 0.0, 1.0);
	let rot = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, p.roll);
	let y_axis = rot * Vec3::Y;
	assert!(
		(y_axis - diag).length() < 1e-3 || (y_axis + diag).length() < 1e-3,
		"kit +Y should align with diagonal, got {y_axis:?}"
	);
	assert!((p.scale.y - 1.0).abs() < 1e-4);
	let want_xz = 0.25 / JOINT_KIT_XZ;
	assert!((p.scale.x - want_xz).abs() < 1e-4);
	assert!((p.scale.z - want_xz).abs() < 1e-4);
}

#[test]
fn edge_thickness_averages_endpoints() {
	let mut c = PanelComplex::rough_stone();
	let a = c.insert_point_thick(Vec3::ZERO, 0.2);
	let b = c.insert_point_thick(Vec3::new(1.0, 0.0, 0.0), 0.6);
	let d = c.insert_point(Vec3::new(0.0, 1.0, 0.0));
	let e = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
	c.add_triangle(a, b, e).add_triangle(a, e, d);
	let shared = c.shared_edges();
	assert_eq!(shared.len(), 1);
	assert!((c.edge_thickness(shared[0].a, shared[0].b).unwrap() - 0.3).abs() < 1e-5);
}

#[test]
fn boundary_only_disjoint_triangles_have_no_shared_edges() {
	let mut c = PanelComplex::rough_stone();
	let a = c.insert_point(Vec3::ZERO);
	let b = c.insert_point(Vec3::new(1.0, 0.0, 0.0));
	let d = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
	let e = c.insert_point(Vec3::new(3.0, 0.0, 0.0));
	let f = c.insert_point(Vec3::new(4.0, 0.0, 0.0));
	let g = c.insert_point(Vec3::new(3.0, 0.0, 1.0));
	c.add_triangle(a, b, d).add_triangle(e, f, g);
	assert!(c.shared_edges().is_empty());
	assert!(c.joint_nodes().is_empty());
}

#[test]
fn non_manifold_edge_omitted_from_shared_and_flagged() {
	let mut c = PanelComplex::rough_stone();
	let a = c.insert_point(Vec3::ZERO);
	let b = c.insert_point(Vec3::new(1.0, 0.0, 0.0));
	let p0 = c.insert_point(Vec3::new(0.0, 1.0, 0.0));
	let p1 = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
	let p2 = c.insert_point(Vec3::new(0.0, -1.0, 0.0));
	c.add_triangle(a, b, p0).add_triangle(a, b, p1).add_triangle(a, b, p2);
	let (shared, non_manifold) = shared_edges(c.triangles());
	assert!(shared.is_empty());
	assert_eq!(non_manifold.len(), 1);
	assert_eq!(non_manifold[0], (a, b));
	let report = c.validate();
	assert!(!report.is_ok());
	assert_eq!(report.non_manifold_edges, vec![(a, b)]);
	assert!(c.joint_nodes().is_empty());
}

#[test]
fn subtle_kink_respects_joint_policy() {
	let mut c = PanelComplex::rough_stone();
	let a0 = c.insert_point(Vec3::new(0.5, 0.0, 0.0));
	let a1 = c.insert_point(Vec3::new(2.5, 0.0, 0.0));
	let b0 = c.insert_point(Vec3::new(0.0, 0.3, 3.0));
	let b1 = c.insert_point(Vec3::new(3.0, 0.0, 3.0));
	c.add_triangle(a0, a1, b1).add_triangle(a0, b1, b0);
	let kink = c.dihedral_kink(c.shared_edges()[0]).expect("kink");
	assert!(kink > 0.1 && kink < 0.2, "expected mild kink, got {kink}");
	assert_eq!(c.joint_nodes().len(), 1);
	c.set_joint_policy(PanelComplexJointPolicy::never());
	assert!(c.joint_nodes().is_empty());
	c.set_joint_policy(PanelComplexJointPolicy::min_dihedral_rad(0.2));
	assert!(c.joint_nodes().is_empty());
}

#[test]
fn remove_point_drops_incident_triangles() {
	let mut c = folded_quad();
	let b1 = PanelPointId(3);
	c.remove_point(b1);
	assert!(c.point(b1).is_none());
	assert!(c.triangles().is_empty());
	assert_eq!(c.points().count(), 3);
}

#[test]
fn owned_with_point_builder() {
	let (c, a) = PanelComplex::rough_stone().with_point(Vec3::ZERO);
	let (c, b) = c.with_point(Vec3::new(1.0, 0.0, 0.0));
	let (mut c, d) = c.with_point(Vec3::new(0.0, 0.0, 1.0));
	c.triangle(a, b, d);
	assert_eq!(c.triangles().len(), 1);
	assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(), 1);
}
