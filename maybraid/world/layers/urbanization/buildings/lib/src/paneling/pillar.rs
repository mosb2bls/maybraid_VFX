//! Square panel-kit pillars and colonnades along a plan segment.
//!
//! Each [`PanelPillar`] is four standing [`Rectangle`] faces around a square
//! footprint. [`PanelPillarLine`] spaces those piers along a world-space run.

use bevy_math::Vec3;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
use crate::paneling::rectangle::Rectangle;

/// Square column of four standing rectangle kits.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelPillar {
	pub style: PanelStyle,
	/// Base center (floor elevation in `y`).
	pub center: Vec3,
	/// Full plan width of the square.
	pub width: f32,
	pub height: f32,
	pub thickness: f32,
	faces: [Rectangle; 4],
}

impl PanelPillar {
	pub fn new(style: PanelStyle, center: Vec3, width: f32, height: f32, thickness: f32) -> Self {
		let width = width.max(1e-3);
		let height = height.max(1e-4);
		let thickness = if thickness > 1e-6 { thickness } else { DEFAULT_PANEL_THICKNESS };
		let half = width * 0.5;
		let y0 = center.y;
		let sw = Vec3::new(center.x - half, y0, center.z - half);
		let se = Vec3::new(center.x + half, y0, center.z - half);
		let ne = Vec3::new(center.x + half, y0, center.z + half);
		let nw = Vec3::new(center.x - half, y0, center.z + half);
		let faces = [
			Rectangle::new(style, sw, se - sw, height, thickness, 0.0),
			Rectangle::new(style, se, ne - se, height, thickness, 0.0),
			Rectangle::new(style, ne, nw - ne, height, thickness, 0.0),
			Rectangle::new(style, nw, sw - nw, height, thickness, 0.0),
		];
		Self { style, center, width, height, thickness, faces }
	}

	pub fn rough_stone(center: Vec3, width: f32, height: f32) -> Self {
		Self::new(PanelStyle::RoughStonework, center, width, height, DEFAULT_PANEL_THICKNESS)
	}

	pub fn faces(&self) -> &[Rectangle; 4] {
		&self.faces
	}
}

impl BuildingComponents for PanelPillar {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for face in &self.faces {
			out.extend(face.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for face in &self.faces {
			out.extend(face.joint_nodes_for_level(level));
		}
		out
	}
}

/// Colonnade of [`PanelPillar`]s along a plan segment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelPillarLine {
	pub pillars: Vec<PanelPillar>,
}

impl PanelPillarLine {
	pub fn new(pillars: impl IntoIterator<Item = PanelPillar>) -> Self {
		Self { pillars: pillars.into_iter().collect() }
	}

	pub fn is_empty(&self) -> bool {
		self.pillars.is_empty()
	}

	pub fn len(&self) -> usize {
		self.pillars.len()
	}

	/// Space pillars from `start` to `end` (base centers) at about `spacing` metres.
	///
	/// Both endpoints are included when the run is long enough for two piers.
	/// A short run still gets one midpoint pier when it can host the square.
	pub fn along(
		style: PanelStyle,
		start: Vec3,
		end: Vec3,
		height: f32,
		width: f32,
		spacing: f32,
		thickness: f32,
	) -> Self {
		let width = width.max(1e-3);
		let spacing = spacing.max(width);
		let delta = end - start;
		let len = delta.length();
		if len < 1e-4 {
			return Self::new([PanelPillar::new(style, start, width, height, thickness)]);
		}
		if len + 1e-3 < width {
			return Self::default();
		}
		if len + 1e-3 < spacing {
			let mid = start + delta * 0.5;
			return Self::new([PanelPillar::new(style, mid, width, height, thickness)]);
		}
		let n_gaps = ((len / spacing).round() as usize).max(1);
		let mut pillars = Vec::with_capacity(n_gaps + 1);
		for i in 0..=n_gaps {
			let t = i as f32 / n_gaps as f32;
			let center = start + delta * t;
			pillars.push(PanelPillar::new(style, center, width, height, thickness));
		}
		Self { pillars }
	}

	pub fn along_rough_stone(
		start: Vec3,
		end: Vec3,
		height: f32,
		width: f32,
		spacing: f32,
	) -> Self {
		Self::along(
			PanelStyle::RoughStonework,
			start,
			end,
			height,
			width,
			spacing,
			DEFAULT_PANEL_THICKNESS,
		)
	}

	/// Drop piers whose XZ center is within `sep` of an already-kept pier.
	pub fn dedup_xz(mut self, sep: f32) -> Self {
		let sep2 = (sep.max(0.0)).powi(2);
		let mut kept = Vec::new();
		for p in self.pillars {
			let dup = kept.iter().any(|k: &PanelPillar| {
				let d = p.center - k.center;
				d.x * d.x + d.z * d.z <= sep2
			});
			if !dup {
				kept.push(p);
			}
		}
		self.pillars = kept;
		self
	}
}

impl BuildingComponents for PanelPillarLine {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for p in &self.pillars {
			out.extend(p.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for p in &self.pillars {
			out.extend(p.joint_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pillar_emits_four_faces() {
		let p = PanelPillar::rough_stone(Vec3::new(1.0, 0.0, 2.0), 0.6, 3.5);
		assert_eq!(p.faces().len(), 4);
		let panels = p.panel_nodes_for_level(LodSceneLevel::High);
		assert_eq!(panels.len(), 4);
	}

	#[test]
	fn line_includes_both_ends() {
		let line = PanelPillarLine::along_rough_stone(
			Vec3::new(-10.0, 0.0, 0.0),
			Vec3::new(10.0, 0.0, 0.0),
			3.0,
			0.5,
			5.0,
		);
		assert_eq!(line.len(), 5);
		assert!((line.pillars[0].center.x + 10.0).abs() < 1e-3);
		assert!((line.pillars.last().unwrap().center.x - 10.0).abs() < 1e-3);
	}

	#[test]
	fn short_run_gets_one_midpoint() {
		let line = PanelPillarLine::along_rough_stone(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(1.2, 0.0, 0.0),
			3.0,
			0.5,
			4.0,
		);
		assert_eq!(line.len(), 1);
		assert!((line.pillars[0].center.x - 0.6).abs() < 1e-3);
	}

	#[test]
	fn dedup_drops_shared_corner() {
		let a = PanelPillar::rough_stone(Vec3::ZERO, 0.5, 3.0);
		let b = PanelPillar::rough_stone(Vec3::new(0.01, 0.0, 0.0), 0.5, 3.0);
		let c = PanelPillar::rough_stone(Vec3::new(4.0, 0.0, 0.0), 0.5, 3.0);
		let line = PanelPillarLine::new([a, b, c]).dedup_xz(0.4);
		assert_eq!(line.len(), 2);
	}
}
