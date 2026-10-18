//! Regular n-gon disk in a plane, optionally with a concentric polygonal hole.
//!
//! Solid fill is a fan from the center. With [`Self::clip`] set to an inner radius,
//! the fill is an annulus of radial quads (two triangles per segment). Present via
//! [`PanelComplex`] → right-triangle panel kits.

use bevy_math::Vec3;
use building_components::panels::PanelStyle;
use building_components::{BuildingComponents, Layers, PanelNode};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelPointId, DEFAULT_PANEL_THICKNESS,
};

/// Minimum segment count for a usable n-gon.
pub const MIN_SEGMENTS: u32 = 3;

/// Default segment count for tower / demo floors.
pub const DEFAULT_SEGMENTS: u32 = 24;

/// Regular n-gon approximation of a circle (optionally an annulus).
#[derive(Debug, Clone, PartialEq)]
pub struct ApproximatedCircle {
	pub style: PanelStyle,
	/// World center of the disk (on the plane).
	pub center: Vec3,
	pub radius: f32,
	pub segments: u32,
	/// Concentric hole radius. [`None`] → solid disk; else annulus when
	/// `0 < clip < radius`.
	pub clip: Option<f32>,
	/// Phase of the first outer vertex (radians, from local \(+X\) toward \(+Z\)
	/// when the normal is \(+Y\)).
	pub start_angle: f32,
	/// Unit plane normal (disk lies in the plane through [`Self::center`]).
	pub normal: Vec3,
	/// World thickness at every vertex (joint / authoring; kit Y stays default).
	pub thickness: f32,
	complex: PanelComplex,
}

impl ApproximatedCircle {
	/// Build eagerly. Degenerate radius / segments → empty complex.
	pub fn new(
		style: PanelStyle,
		center: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
	) -> Self {
		Self::with_frame(
			style,
			center,
			radius,
			segments,
			clip,
			0.0,
			Vec3::Y,
			DEFAULT_PANEL_THICKNESS,
		)
	}

	/// Horizontal disk in \(XZ\) (normal \(+Y\)), typical for floors.
	pub fn horizontal(
		style: PanelStyle,
		center_xz: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
	) -> Self {
		Self::new(style, center_xz, radius, segments, clip)
	}

	pub fn rough_stone(center: Vec3, radius: f32, segments: u32, clip: Option<f32>) -> Self {
		Self::new(PanelStyle::RoughStonework, center, radius, segments, clip)
	}

	pub fn rough_stone_horizontal(
		center_xz: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
	) -> Self {
		Self::horizontal(PanelStyle::RoughStonework, center_xz, radius, segments, clip)
	}

	pub fn with_frame(
		style: PanelStyle,
		center: Vec3,
		radius: f32,
		segments: u32,
		clip: Option<f32>,
		start_angle: f32,
		normal: Vec3,
		thickness: f32,
	) -> Self {
		let radius = radius.max(0.0);
		let segments = segments.max(MIN_SEGMENTS);
		let thickness = thickness.max(1e-4);
		let normal = normal.normalize_or_zero();
		let clip = clip.and_then(|r| {
			let r = r.max(0.0);
			if r < 1e-6 || r >= radius - 1e-6 {
				None
			} else {
				Some(r)
			}
		});
		let complex = build_complex(
			style,
			center,
			radius,
			segments,
			clip,
			start_angle,
			normal,
			thickness,
			PanelComplexJointPolicy::never(),
		);
		Self { style, center, radius, segments, clip, start_angle, normal, thickness, complex }
	}

	pub fn with_start_angle(mut self, start_angle: f32) -> Self {
		self.start_angle = start_angle;
		self.rebuild()
	}

	pub fn with_normal(mut self, normal: Vec3) -> Self {
		self.normal = normal.normalize_or_zero();
		self.rebuild()
	}

	pub fn with_thickness(mut self, thickness: f32) -> Self {
		self.thickness = thickness.max(1e-4);
		self.rebuild()
	}

	pub fn with_clip(mut self, clip: Option<f32>) -> Self {
		self.clip = clip.and_then(|r| {
			let r = r.max(0.0);
			if r < 1e-6 || r >= self.radius - 1e-6 {
				None
			} else {
				Some(r)
			}
		});
		self.rebuild()
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}

	fn rebuild(self) -> Self {
		Self::with_frame(
			self.style,
			self.center,
			self.radius,
			self.segments,
			self.clip,
			self.start_angle,
			self.normal,
			self.thickness,
		)
	}
}

impl AsRef<PanelComplex> for ApproximatedCircle {
	fn as_ref(&self) -> &PanelComplex {
		&self.complex
	}
}

impl From<ApproximatedCircle> for PanelComplex {
	fn from(value: ApproximatedCircle) -> Self {
		value.into_complex()
	}
}

impl BuildingComponents for ApproximatedCircle {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.complex.panel_nodes_for_level(level)
	}
}

fn build_complex(
	style: PanelStyle,
	center: Vec3,
	radius: f32,
	segments: u32,
	clip: Option<f32>,
	start_angle: f32,
	normal: Vec3,
	thickness: f32,
	joint_policy: PanelComplexJointPolicy,
) -> PanelComplex {
	let mut complex = PanelComplex::new(style).with_joint_policy(joint_policy);
	if radius < 1e-6 || normal.length_squared() < 1e-12 {
		return complex;
	}

	let (e0, e1) = orthonormal_basis(normal);
	let n = segments as usize;
	let outer = ring_points(center, radius, n, start_angle, e0, e1);
	let outer_ids: Vec<PanelPointId> =
		outer.iter().map(|&p| complex.insert_point_thick(p, thickness)).collect();

	if let Some(inner_r) = clip {
		let inner = ring_points(center, inner_r, n, start_angle, e0, e1);
		let inner_ids: Vec<PanelPointId> =
			inner.iter().map(|&p| complex.insert_point_thick(p, thickness)).collect();
		for i in 0..n {
			let i1 = (i + 1) % n;
			// Two tris per radial quad; winding matches +normal.
			complex.add_triangle(outer_ids[i], outer_ids[i1], inner_ids[i1]);
			complex.add_triangle(outer_ids[i], inner_ids[i1], inner_ids[i]);
		}
	} else {
		let c_id = complex.insert_point_thick(center, thickness);
		for i in 0..n {
			let i1 = (i + 1) % n;
			complex.add_triangle(c_id, outer_ids[i], outer_ids[i1]);
		}
	}
	complex
}

fn orthonormal_basis(normal: Vec3) -> (Vec3, Vec3) {
	let n = normal.normalize_or_zero();
	let helper = if n.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
	let e0 = n.cross(helper).normalize_or_zero();
	let e1 = n.cross(e0).normalize_or_zero();
	(e0, e1)
}

fn ring_points(
	center: Vec3,
	radius: f32,
	segments: usize,
	start_angle: f32,
	e0: Vec3,
	e1: Vec3,
) -> Vec<Vec3> {
	let mut pts = Vec::with_capacity(segments);
	for i in 0..segments {
		let a = start_angle + std::f32::consts::TAU * (i as f32) / (segments as f32);
		pts.push(center + e0 * (radius * a.cos()) + e1 * (radius * a.sin()));
	}
	pts
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_disk_has_one_triangle_per_segment() {
		let disk = ApproximatedCircle::rough_stone(Vec3::ZERO, 2.0, 8, None);
		assert_eq!(disk.as_complex().triangles().len(), 8);
	}

	#[test]
	fn annulus_has_two_triangles_per_segment() {
		let disk = ApproximatedCircle::rough_stone(Vec3::ZERO, 2.0, 12, Some(0.5));
		assert_eq!(disk.as_complex().triangles().len(), 24);
		assert!(disk.clip.is_some());
	}

	#[test]
	fn oversized_clip_becomes_solid() {
		let disk = ApproximatedCircle::rough_stone(Vec3::ZERO, 1.0, 6, Some(2.0));
		assert!(disk.clip.is_none());
		assert_eq!(disk.as_complex().triangles().len(), 6);
	}

	#[test]
	fn horizontal_emits_panel_nodes() {
		let disk = ApproximatedCircle::rough_stone_horizontal(
			Vec3::new(1.0, 0.0, 2.0),
			3.0,
			16,
			Some(1.0),
		);
		let nodes = disk.panel_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 32);
	}
}
