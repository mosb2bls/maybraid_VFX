//! Oriented ordinary rectangle kits ([`PanelGeometry::Rectangle`]).
//!
//! Authored by lowest-edge vector (length / slope / yaw), height, thickness, and
//! roll (`0` ⇒ top toward world `+Y`). [`ClippedRectangle`] punches an opening via
//! [`RectInset`] margins — a frame of other rectangle kits.

use bevy_math::Vec3;
use building_components::panels::{PanelGeometry, PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
use crate::paneling::rect_fit::{fallback_oriented, orient_rectangle, OrientedRect, RectInset};

/// Solid oriented rectangle → one [`PanelGeometry::Rectangle`] kit.
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
	pub style: PanelStyle,
	pub origin: Vec3,
	/// Path of the lowest edge (length = `|edge|`).
	pub edge: Vec3,
	pub height: f32,
	pub thickness: f32,
	/// `0` ⇒ top toward world `+Y`.
	pub roll: f32,
	pub oriented: OrientedRect,
	pub panel: PanelNode,
}

impl Rectangle {
	pub fn new(
		style: PanelStyle,
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
	) -> Self {
		let height = height.max(1e-4);
		let thickness = if thickness > 1e-6 { thickness } else { DEFAULT_PANEL_THICKNESS };
		let oriented = orient_rectangle(origin, edge, height, roll)
			.unwrap_or_else(|| fallback_oriented(origin, edge, height));
		let panel =
			PanelNode::new(style, PanelGeometry::rectangle(), oriented.solid_placement(thickness));
		Self { style, origin, edge, height, thickness, roll, oriented, panel }
	}

	pub fn rough_stone(origin: Vec3, edge: Vec3, height: f32, thickness: f32, roll: f32) -> Self {
		Self::new(PanelStyle::RoughStonework, origin, edge, height, thickness, roll)
	}

	pub fn shepherds_thatch(
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, origin, edge, height, thickness, roll)
	}

	pub fn panel_node(&self) -> &PanelNode {
		&self.panel
	}
}

impl BuildingComponents for Rectangle {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::from_free(vec![self.panel.clone()])
	}
}

/// Oriented rectangle with an inset opening framed by rectangle kits.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRectangle {
	pub style: PanelStyle,
	pub origin: Vec3,
	pub edge: Vec3,
	pub height: f32,
	pub thickness: f32,
	pub roll: f32,
	pub oriented: OrientedRect,
	pub inset: RectInset,
	pub panels: Vec<PanelNode>,
}

impl ClippedRectangle {
	pub fn new(
		style: PanelStyle,
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
		inset: RectInset,
	) -> Self {
		let height = height.max(1e-4);
		let thickness = if thickness > 1e-6 { thickness } else { DEFAULT_PANEL_THICKNESS };
		let oriented = orient_rectangle(origin, edge, height, roll)
			.unwrap_or_else(|| fallback_oriented(origin, edge, height));
		let panels = inset
			.frame_pieces(oriented.width, oriented.depth)
			.into_iter()
			.map(|(u0, v0, w, d)| {
				PanelNode::new(
					style,
					PanelGeometry::rectangle(),
					oriented.panel_placement(u0, v0, w, d, thickness),
				)
			})
			.collect();
		Self { style, origin, edge, height, thickness, roll, oriented, inset, panels }
	}

	pub fn rough_stone(
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
		inset: RectInset,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, origin, edge, height, thickness, roll, inset)
	}

	pub fn shepherds_thatch(
		origin: Vec3,
		edge: Vec3,
		height: f32,
		thickness: f32,
		roll: f32,
		inset: RectInset,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, origin, edge, height, thickness, roll, inset)
	}

	pub fn panels(&self) -> &[PanelNode] {
		&self.panels
	}
}

impl BuildingComponents for ClippedRectangle {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::from_free(self.panels.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_is_one_rectangle_kit() {
		let r = Rectangle::rough_stone(Vec3::ZERO, Vec3::new(0.0, 0.0, 1.0), 2.0, 0.75, 0.0);
		assert!(matches!(r.panel.geometry, PanelGeometry::Rectangle(_)));
		assert!((r.panel.placement.scale.x - 2.0).abs() < 1e-3);
		assert!((r.panel.placement.scale.z - 1.0).abs() < 1e-3);
	}

	#[test]
	fn inset_emits_four_rectangle_kits() {
		let r = ClippedRectangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(0.0, 0.0, 1.0),
			2.0,
			0.75,
			0.0,
			RectInset::uniform(0.25),
		);
		assert_eq!(r.panels().len(), 4);
		assert!(r.panels().iter().all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
}
