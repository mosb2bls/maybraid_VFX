//! Best-fit ordinary rectangle kits ([`FittedRectangle`]) for a ruled bay.
//!
//! [`ClippedFittedRectangle`] punches an opening via [`RectInset`] margins — a frame of
//! other rectangle kits — not a polygonal world clip / earcut path.

use bevy_math::Vec3;
use building_components::panels::{PanelGeometry, PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_complex::{PanelPoint, DEFAULT_PANEL_THICKNESS};
use crate::paneling::rect_fit::{fit_rectangle, FittedRect, RectInset};

/// Solid best-fit rectangle → one [`PanelGeometry::Rectangle`] kit.
#[derive(Debug, Clone, PartialEq)]
pub struct FittedRectangle {
	pub style: PanelStyle,
	/// Authored (possibly skew) corners.
	pub a0: PanelPoint,
	pub a1: PanelPoint,
	pub b0: PanelPoint,
	pub b1: PanelPoint,
	pub fitted: FittedRect,
	pub panel: PanelNode,
}

impl FittedRectangle {
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
		let fitted = fit_rectangle(a0.position, a1.position, b0.position, b1.position)
			.unwrap_or_else(|| {
				fallback_fitted_rect(a0.position, a1.position, b0.position, b1.position)
			});
		let thickness = mean_thickness([a0, a1, b0, b1]);
		let panel =
			PanelNode::new(style, PanelGeometry::rectangle(), fitted.solid_placement(thickness));
		Self { style, a0, a1, b0, b1, fitted, panel }
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1)
	}

	pub fn panel_node(&self) -> &PanelNode {
		&self.panel
	}

	/// Mean authored thickness (also `panel.placement.scale.y`).
	pub fn thickness(&self) -> f32 {
		self.panel.placement.scale.y
	}

	/// Thickness along the leading generator (`a0`–`b0`).
	pub fn start_thickness(&self) -> f32 {
		((self.a0.thickness + self.b0.thickness) * 0.5).max(1e-4)
	}

	/// Thickness along the trailing generator (`a1`–`b1`).
	pub fn end_thickness(&self) -> f32 {
		((self.a1.thickness + self.b1.thickness) * 0.5).max(1e-4)
	}
}

impl BuildingComponents for FittedRectangle {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::from_free(vec![self.panel.clone()])
	}
}

/// Best-fit rectangle with an inset opening framed by rectangle kits.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedFittedRectangle {
	pub style: PanelStyle,
	pub a0: PanelPoint,
	pub a1: PanelPoint,
	pub b0: PanelPoint,
	pub b1: PanelPoint,
	pub fitted: FittedRect,
	pub inset: RectInset,
	pub panels: Vec<PanelNode>,
}

impl ClippedFittedRectangle {
	pub fn new(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		inset: RectInset,
	) -> Self {
		let a0 = a0.into();
		let a1 = a1.into();
		let b0 = b0.into();
		let b1 = b1.into();
		let fitted = fit_rectangle(a0.position, a1.position, b0.position, b1.position)
			.unwrap_or_else(|| {
				fallback_fitted_rect(a0.position, a1.position, b0.position, b1.position)
			});
		let thickness = mean_thickness([a0, a1, b0, b1]);
		let panels = inset
			.frame_pieces(fitted.width, fitted.depth)
			.into_iter()
			.map(|(u0, v0, w, d)| {
				PanelNode::new(
					style,
					PanelGeometry::rectangle(),
					fitted.panel_placement(u0, v0, w, d, thickness),
				)
			})
			.collect();
		Self { style, a0, a1, b0, b1, fitted, inset, panels }
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		inset: RectInset,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1, inset)
	}

	pub fn shepherds_thatch(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		inset: RectInset,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a0, a1, b0, b1, inset)
	}

	pub fn panels(&self) -> &[PanelNode] {
		&self.panels
	}

	pub fn thickness(&self) -> f32 {
		mean_thickness([self.a0, self.a1, self.b0, self.b1])
	}

	pub fn start_thickness(&self) -> f32 {
		((self.a0.thickness + self.b0.thickness) * 0.5).max(1e-4)
	}

	pub fn end_thickness(&self) -> f32 {
		((self.a1.thickness + self.b1.thickness) * 0.5).max(1e-4)
	}
}

impl BuildingComponents for ClippedFittedRectangle {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::from_free(self.panels.clone())
	}
}

fn mean_thickness(pts: [PanelPoint; 4]) -> f32 {
	let t = pts.iter().map(|p| p.thickness).sum::<f32>() * 0.25;
	if t > 1e-6 {
		t
	} else {
		DEFAULT_PANEL_THICKNESS
	}
}

fn fallback_fitted_rect(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> FittedRect {
	let e0 = (a1 - a0).normalize_or_zero();
	let e1 = (b0 - a0).normalize_or_zero();
	let normal = e0.cross(e1).normalize_or_zero();
	FittedRect {
		a0,
		a1,
		b0,
		b1,
		e0,
		e1,
		normal,
		width: (a1 - a0).length().max(1e-4),
		depth: (b0 - a0).length().max(1e-4),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_is_one_rectangle_kit() {
		let r = FittedRectangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
			Vec3::new(2.0, 0.0, 1.0),
		);
		assert!(matches!(r.panel.geometry, PanelGeometry::Rectangle(_)));
		assert!((r.panel.placement.scale.x - 2.0).abs() < 1e-3);
		assert!((r.panel.placement.scale.z - 1.0).abs() < 1e-3);
	}

	#[test]
	fn inset_emits_four_rectangle_kits() {
		let r = ClippedFittedRectangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
			Vec3::new(2.0, 0.0, 1.0),
			RectInset::uniform(0.25),
		);
		assert_eq!(r.panels().len(), 4);
		assert!(r.panels().iter().all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
}
