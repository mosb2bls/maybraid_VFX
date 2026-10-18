//! Circular ring storey shell: outer + inner arc walls + annulus floor.
//!
//! Walls are full 360° [`ClippedArcSweep`] rings. Connectable openings map to
//! angular \(t\) clips on the nearest ring (by radial distance). Floor / ceiling
//! use [`ApproximatedCircle`] with a concentric clip (= inner radius).
//!
//! **Slabs (v1):** `cuts_slab` openings that erase enough of the annulus omit the
//! whole piece; finer polygonal atrium bites are out of scope.

mod openings;
mod slabs;

#[cfg(test)]
mod tests;

use bevy_math::Vec3;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::partitions::PartitionNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::arcs::ClippedArcSweep;
use crate::openings::{MappedOpenings, Openings};
use crate::paneling::approximated_circle::{ApproximatedCircle, DEFAULT_SEGMENTS};
use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;

/// Horizontal storey slab presentation for towering ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircRingFloorSlab {
	None,
	Solid,
}

impl Default for CircRingFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for a [`CircRingFloor`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct CircRingFloorParams {
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	pub outer_radius: f32,
	/// Courtyard radius. Must satisfy `0 < inner < outer`.
	pub inner_radius: f32,
	pub storey_height: f32,
	pub openings: Openings,
	pub floor: CircRingFloorSlab,
	pub ceiling: CircRingFloorSlab,
	/// Style for annulus floor / ceiling kits.
	pub style: PanelStyle,
	pub joint_thickness: f32,
	pub segments: u32,
}

impl Default for CircRingFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			outer_radius: 5.0,
			inner_radius: 2.5,
			storey_height: 3.0,
			openings: Openings::new(),
			floor: CircRingFloorSlab::None,
			ceiling: CircRingFloorSlab::None,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
			segments: DEFAULT_SEGMENTS,
		}
	}
}

impl CircRingFloorParams {
	pub fn new(center_xz: Vec3, outer_radius: f32, inner_radius: f32, storey_height: f32) -> Self {
		Self { center_xz, outer_radius, inner_radius, storey_height, ..Self::default() }
	}

	pub fn floor(mut self, floor: CircRingFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: CircRingFloorSlab) -> Self {
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

	pub fn segments(mut self, segments: u32) -> Self {
		self.segments = segments;
		self
	}

	pub fn build(self) -> CircRingFloor {
		CircRingFloor::from_params(self)
	}
}

/// One circular-ring storey: outer/inner walls + optional annulus slabs.
#[derive(Debug, Clone, PartialEq)]
pub struct CircRingFloor {
	params: CircRingFloorParams,
	outer_wall: ClippedArcSweep,
	inner_wall: ClippedArcSweep,
	floor: Option<ApproximatedCircle>,
	ceiling: Option<ApproximatedCircle>,
	openings: Openings,
	mapped: MappedOpenings,
}

impl CircRingFloor {
	pub fn new(params: CircRingFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: CircRingFloorParams) -> Self {
		let (outer_radius, inner_radius) = sanitize_radii(params.outer_radius, params.inner_radius);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params =
			CircRingFloorParams { center_xz, outer_radius, inner_radius, storey_height, ..params };

		let (outer_wall, inner_wall, openings, mapped) = params.resolve_walls();
		let floor = params.resolve_slab(params.floor, center_xz);
		let ceiling = params.resolve_slab(params.ceiling, center_xz + Vec3::Y * storey_height);

		Self { params, outer_wall, inner_wall, floor, ceiling, openings, mapped }
	}

	pub fn params(&self) -> &CircRingFloorParams {
		&self.params
	}

	pub fn outer_wall(&self) -> &ClippedArcSweep {
		&self.outer_wall
	}

	pub fn inner_wall(&self) -> &ClippedArcSweep {
		&self.inner_wall
	}

	pub fn has_floor(&self) -> bool {
		self.floor.is_some()
	}

	pub fn has_ceiling(&self) -> bool {
		self.ceiling.is_some()
	}

	pub fn floor_circle(&self) -> Option<&ApproximatedCircle> {
		self.floor.as_ref()
	}
}

impl BuildingComponents for CircRingFloor {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(f) = &self.floor {
			out.extend(f.panel_nodes_for_level(level));
		}
		if let Some(c) = &self.ceiling {
			out.extend(c.panel_nodes_for_level(level));
		}
		out
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = Layers::new();
		out.extend(self.outer_wall.partition_nodes_for_level(level));
		out.extend(self.inner_wall.partition_nodes_for_level(level));
		out
	}
}

fn sanitize_radii(outer: f32, inner: f32) -> (f32, f32) {
	let outer = outer.max(1e-3);
	let mut inner = inner.max(1e-4);
	const MIN_BAND: f32 = 0.25;
	if inner >= outer - MIN_BAND {
		inner = (outer - MIN_BAND).max(1e-4);
	}
	(outer, inner)
}
