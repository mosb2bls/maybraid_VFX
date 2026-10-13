//! Closed rectangular strip around a sampled ground height.
//!
//! Every station shares one base below the lowest sample, so each bay's
//! bottom edge is horizontal and its panel stands vertical. Lower ground
//! simply buries more of the wall; the top stays level at `plaza + clearance`.

use bevy_math::Vec2;
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use crate::paneling::{RectangularStrip, RectangularStripNode, DEFAULT_PANEL_THICKNESS};

/// Sink below the lowest sample so a coarse collider never shows a gap.
const PERIMETER_BURY_M: f32 = 0.5;

/// Level-topped perimeter built from [`RectangularStrip`] bays.
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainPerimeterWall {
	strip: RectangularStrip,
	material: Option<MaterialRef>,
}

impl TerrainPerimeterWall {
	/// Stations around an axis-aligned XZ rectangle, excluding the repeated close.
	pub fn sample_rectangle(min: Vec2, max: Vec2, step: f32) -> Vec<Vec2> {
		let (min, max) = (min.min(max), min.max(max));
		let mut points = Vec::new();
		push_edge(&mut points, Vec2::new(min.x, min.y), Vec2::new(max.x, min.y), step);
		push_edge(&mut points, Vec2::new(max.x, min.y), Vec2::new(max.x, max.y), step);
		push_edge(&mut points, Vec2::new(max.x, max.y), Vec2::new(min.x, max.y), step);
		push_edge(&mut points, Vec2::new(min.x, max.y), Vec2::new(min.x, min.y), step);
		points
	}

	/// `terrain_y[i]` is the ground height at `samples[i]`. The top sits
	/// `clearance` above `plaza_y`. The strip repeats the first station so the
	/// last bay closes the rectangle.
	pub fn from_samples(samples: &[Vec2], terrain_y: &[f32], plaza_y: f32, clearance: f32) -> Self {
		let clearance = clearance.max(0.5);
		let n = samples.len().min(terrain_y.len());
		let lowest = terrain_y[..n].iter().copied().fold(plaza_y, f32::min);
		let base = lowest - PERIMETER_BURY_M;
		let height = plaza_y + clearance - base;
		let mut nodes: Vec<_> = samples[..n]
			.iter()
			.map(|xz| {
				RectangularStripNode::new(
					bevy_math::Vec3::new(xz.x, base, xz.y),
					height,
					DEFAULT_PANEL_THICKNESS,
					0.0,
				)
			})
			.collect();
		if let Some(first) = nodes.first().copied() {
			nodes.push(first);
		}
		Self {
			strip: RectangularStrip::from_nodes(PanelStyle::RoughStonework, nodes),
			material: None,
		}
	}

	/// Shade every panel with `material` instead of the kit's baked look.
	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn strip(&self) -> &RectangularStrip {
		&self.strip
	}
}

fn push_edge(out: &mut Vec<Vec2>, from: Vec2, to: Vec2, step: f32) {
	let delta = to - from;
	let len = delta.length();
	if len <= 1e-4 {
		return;
	}
	let n = (len / step.max(0.5)).ceil().max(1.0) as usize;
	for i in 0..n {
		out.push(from + delta * (i as f32 / n as f32));
	}
}

impl BuildingComponents for TerrainPerimeterWall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let panels = self.strip.panel_nodes_for_level(level);
		match &self.material {
			Some(material) => panels.with_material(material.clone()),
			None => panels,
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.strip.joint_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn closed_strip_is_level_topped_and_vertical_on_a_slope() {
		let samples = TerrainPerimeterWall::sample_rectangle(
			Vec2::new(-10.0, -8.0),
			Vec2::new(10.0, 8.0),
			8.0,
		);
		assert!(samples.len() >= 4);
		let terrain: Vec<f32> = samples.iter().map(|p| p.x * 0.25).collect();
		let wall = TerrainPerimeterWall::from_samples(&samples, &terrain, 5.0, 20.0);
		let nodes = wall.strip().nodes();
		assert_eq!(nodes.len(), samples.len() + 1);
		assert_eq!(nodes.first().map(|n| n.position), nodes.last().map(|n| n.position));
		let lowest = terrain.iter().copied().fold(5.0, f32::min);
		for node in nodes {
			assert!((node.position.y - (lowest - PERIMETER_BURY_M)).abs() < 1e-4);
			assert!((node.position.y + node.height - 25.0).abs() < 1e-3);
			assert_eq!(node.roll, 0.0);
		}
		assert!(!wall.strip().bays().is_empty());
	}

	#[test]
	fn material_stamps_every_panel() {
		let samples =
			TerrainPerimeterWall::sample_rectangle(Vec2::splat(-8.0), Vec2::splat(8.0), 8.0);
		let terrain = vec![0.0; samples.len()];
		let wall = TerrainPerimeterWall::from_samples(&samples, &terrain, 0.0, 20.0)
			.with_material(MaterialRef::named("stone"));
		let panels = wall.panel_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!panels.is_empty());
		assert!(panels.iter().all(|panel| panel.material.is_some()));
	}
}
