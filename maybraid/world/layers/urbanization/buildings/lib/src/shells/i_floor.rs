//! Orthonormal I / T / U / L / Z storey shell from a central bar + optional flanges.
//!
//! Outer walls are per-edge [`ClippedRectangularStrip`] rectangle kits. Floor /
//! ceiling are unions of axis-aligned rectangle pieces. Openings fit to authored
//! AABB positions on the hit edge / slab piece.

mod geometry;
mod openings;
mod slabs;

#[cfg(test)]
mod tests;

use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::openings::{MappedOpenings, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;

use crate::shells::ortho::WallEdge;

pub use geometry::PlanAabb as IFloorPlanRect;

/// Horizontal storey slab presentation for towering ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IFloorSlab {
	None,
	Solid,
}

impl Default for IFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for an [`IFloor`] shell.
///
/// Flange lengths extend the central rectangle along ±X at the top (−Z) and
/// bottom (+Z) ends. Omitting flanges yields T / U / L / Z / I footprints.
#[derive(Debug, Clone, PartialEq)]
pub struct IFloorParams {
	pub center_xz: Vec3,
	pub top_left_length: Option<f32>,
	pub top_right_length: Option<f32>,
	/// Central bar `(width_x, depth_z)`.
	pub central_rectangle: Vec2,
	pub bottom_left_length: Option<f32>,
	pub bottom_right_length: Option<f32>,
	/// Flange bar depth along ±Z. `None` ⇒ same as central width (classic I).
	pub flange_thickness: Option<f32>,
	pub storey_height: f32,
	pub openings: Openings,
	pub floor: IFloorSlab,
	pub ceiling: IFloorSlab,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for IFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			top_left_length: Some(2.0),
			top_right_length: Some(2.0),
			central_rectangle: Vec2::new(2.0, 6.0),
			bottom_left_length: Some(2.0),
			bottom_right_length: Some(2.0),
			flange_thickness: None,
			storey_height: 3.0,
			openings: Openings::new(),
			floor: IFloorSlab::None,
			ceiling: IFloorSlab::None,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}
}

impl IFloorParams {
	pub fn new(center_xz: Vec3, central_rectangle: Vec2, storey_height: f32) -> Self {
		Self { center_xz, central_rectangle, storey_height, ..Self::default() }
	}

	pub fn top_left_length(mut self, length: Option<f32>) -> Self {
		self.top_left_length = length;
		self
	}

	pub fn top_right_length(mut self, length: Option<f32>) -> Self {
		self.top_right_length = length;
		self
	}

	pub fn bottom_left_length(mut self, length: Option<f32>) -> Self {
		self.bottom_left_length = length;
		self
	}

	pub fn bottom_right_length(mut self, length: Option<f32>) -> Self {
		self.bottom_right_length = length;
		self
	}

	pub fn flange_thickness(mut self, thickness: Option<f32>) -> Self {
		self.flange_thickness = thickness;
		self
	}

	pub fn floor(mut self, floor: IFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: IFloorSlab) -> Self {
		self.ceiling = ceiling;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	pub fn joint_thickness(mut self, thickness: f32) -> Self {
		self.joint_thickness = thickness;
		self
	}

	pub fn build(self) -> IFloor {
		IFloor::from_params(self)
	}

	/// Primary I-plan rectangles (stem + optional flange bars) used for packing.
	pub fn plan_rects(&self) -> Vec<IFloorPlanRect> {
		self.resolve_geometry().slab_rects
	}

	/// Outer wall edges used for packing facade openings before build.
	pub fn wall_edges(&self) -> Vec<WallEdge> {
		self.resolve_geometry().edges
	}
}

/// One I-plan storey: rectilinear outer walls + multi-rect slabs.
#[derive(Debug, Clone, PartialEq)]
pub struct IFloor {
	params: IFloorParams,
	walls: Vec<ClippedRectangularStrip>,
	edges: Vec<WallEdge>,
	floor_pieces: Vec<ISlabPiece>,
	ceiling_pieces: Vec<ISlabPiece>,
	openings: Openings,
	mapped: MappedOpenings,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ISlabPiece {
	Solid(FittedRectangle),
	Clipped(ClippedFittedRectangle),
}

impl ISlabPiece {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Solid(r) => r.panel_nodes_for_level(level),
			Self::Clipped(r) => r.panel_nodes_for_level(level),
		}
	}
}

impl IFloor {
	pub fn new(params: IFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: IFloorParams) -> Self {
		let central =
			Vec2::new(params.central_rectangle.x.max(1e-4), params.central_rectangle.y.max(1e-4));
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params =
			IFloorParams { center_xz, central_rectangle: central, storey_height, ..params };

		let geom = params.resolve_geometry();
		let (walls, openings, mapped) = params.resolve_walls(&geom.edges);
		let floor_pieces = params.resolve_slab_pieces(params.floor, &geom.slab_rects, geom.y0);
		let ceiling_pieces =
			params.resolve_slab_pieces(params.ceiling, &geom.slab_rects, geom.y0 + storey_height);

		Self { params, walls, edges: geom.edges, floor_pieces, ceiling_pieces, openings, mapped }
	}

	pub fn params(&self) -> &IFloorParams {
		&self.params
	}

	pub fn walls(&self) -> &[ClippedRectangularStrip] {
		&self.walls
	}

	pub fn edges(&self) -> &[WallEdge] {
		&self.edges
	}

	pub fn wall_count(&self) -> usize {
		self.walls.len()
	}

	pub fn has_floor(&self) -> bool {
		!self.floor_pieces.is_empty()
	}

	pub fn has_ceiling(&self) -> bool {
		!self.ceiling_pieces.is_empty()
	}

	/// Primary I-plan rectangles (stem + optional flange bars) used for packing.
	pub fn plan_rects(&self) -> Vec<IFloorPlanRect> {
		self.params.plan_rects()
	}
}

impl BuildingComponents for IFloor {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		for p in &self.floor_pieces {
			out.extend(p.panel_nodes_for_level(level));
		}
		for p in &self.ceiling_pieces {
			out.extend(p.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		out
	}
}
