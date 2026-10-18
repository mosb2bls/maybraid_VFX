//! Orthonormal rectangular storey shell: four walls + optional floor / ceiling.
//!
//! Walls use [`RectangularNTube`] (ordinary rectangle kits). Openings fit to the
//! authored AABB position on the hit face / slab — not centered approximations.
//!
//! **Walls:** `Passage` / `Aperture` (and wall-hitting `Shaft`) map to a positioned
//! [`RectInset`] on the nearest side; largest face-aligned extent wins per side.
//! **Slabs:** only [`OpeningLabel::cuts_slab`] labels cut Solid floor / ceiling.

mod openings;
mod slabs;

#[cfg(test)]
mod tests;

use bevy_math::bounding::Aabb2d;
use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;

use crate::openings::{MappedOpenings, Openings};
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
use crate::paneling::rectangular_n_tube::{
	RectangularNTube, RectangularNTubeCorner, RectangularNTubeStation,
};
use crate::paneling::RectInset;

use crate::shells::ortho::{OrthoSide, PlanRect};

pub use openings::RectFloorSide;

/// Horizontal storey slab presentation for towering ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectFloorSlab {
	None,
	Solid,
}

impl Default for RectFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for a [`RectFloor`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct RectFloorParams {
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	/// Full width (X) / depth (Z).
	pub footprint: Vec2,
	pub storey_height: f32,
	pub openings: Openings,
	pub floor: RectFloorSlab,
	pub ceiling: RectFloorSlab,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for RectFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			footprint: Vec2::new(8.0, 6.0),
			storey_height: 3.0,
			openings: Openings::new(),
			floor: RectFloorSlab::None,
			ceiling: RectFloorSlab::None,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}
}

impl RectFloorParams {
	pub fn new(center_xz: Vec3, footprint: Vec2, storey_height: f32) -> Self {
		Self { center_xz, footprint, storey_height, ..Self::default() }
	}

	pub fn floor(mut self, floor: RectFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: RectFloorSlab) -> Self {
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

	pub fn build(self) -> RectFloor {
		RectFloor::from_params(self)
	}

	pub(super) fn plan(&self) -> PlanRect {
		PlanRect::new(self.center_xz, self.footprint.x, self.footprint.y)
	}
}

/// One rectangular storey: N-tube walls + optional floor / ceiling slabs.
#[derive(Debug, Clone, PartialEq)]
pub struct RectFloor {
	params: RectFloorParams,
	walls: RectangularNTube,
	floor: Option<RectFloorSlabGeom>,
	ceiling: Option<RectFloorSlabGeom>,
	openings: Openings,
	mapped: MappedOpenings,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum RectFloorSlabGeom {
	Solid(FittedRectangle),
	Clipped(ClippedFittedRectangle),
}

impl RectFloorSlabGeom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Solid(r) => r.panel_nodes_for_level(level),
			Self::Clipped(r) => r.panel_nodes_for_level(level),
		}
	}
}

impl RectFloor {
	pub fn new(params: RectFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: RectFloorParams) -> Self {
		let footprint = Vec2::new(params.footprint.x.max(1e-4), params.footprint.y.max(1e-4));
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params = RectFloorParams { center_xz, footprint, storey_height, ..params };

		let plan = params.plan();
		let (side_insets, openings, mapped) = params.resolve_wall_openings(plan);
		let face_insets: Vec<Vec<Option<RectInset>>> = OrthoSide::all()
			.into_iter()
			.map(|side| vec![side_insets[side.face_index()]])
			.collect();

		let t = params.joint_thickness.max(1e-4);
		let y0 = plan.y;
		let y1 = plan.y + storey_height;
		let floor_station = station_at(plan, y0, t);
		let ceil_station = station_at(plan, y1, t);
		let walls = RectangularNTube::from_stations_with_insets(
			params.style,
			[floor_station, ceil_station],
			face_insets,
		);

		let floor = params.resolve_slab(params.floor, plan);
		let ceiling = params.resolve_slab(
			params.ceiling,
			PlanRect::new(
				Vec3::new(plan.center.x, y1, plan.center.z),
				plan.full_x(),
				plan.full_z(),
			),
		);

		Self { params, walls, floor, ceiling, openings, mapped }
	}

	pub fn params(&self) -> &RectFloorParams {
		&self.params
	}

	pub fn walls(&self) -> &RectangularNTube {
		&self.walls
	}

	pub fn has_floor(&self) -> bool {
		self.floor.is_some()
	}

	pub fn has_ceiling(&self) -> bool {
		self.ceiling.is_some()
	}
}

fn station_at(plan: PlanRect, y: f32, thickness: f32) -> RectangularNTubeStation {
	let p = PlanRect { y, ..plan };
	RectangularNTubeStation::new([
		RectangularNTubeCorner::new(p.sw(), thickness),
		RectangularNTubeCorner::new(p.se(), thickness),
		RectangularNTubeCorner::new(p.ne(), thickness),
		RectangularNTubeCorner::new(p.nw(), thickness),
	])
}

impl BuildingComponents for RectFloor {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.walls.panel_nodes_for_level(level);
		if let Some(floor) = &self.floor {
			out.extend(floor.panel_nodes_for_level(level));
		}
		if let Some(ceiling) = &self.ceiling {
			out.extend(ceiling.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.walls.joint_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let p = self.params();
		let half = p.footprint * 0.5;
		let xz = Aabb2d {
			min: Vec2::new(p.center_xz.x - half.x, p.center_xz.z - half.y),
			max: Vec2::new(p.center_xz.x + half.x, p.center_xz.z + half.y),
		};
		Some(BuildingStructuralLodProbe::from_volumes([MassingVolume::cuboid(
			xz,
			p.center_xz.y,
			p.storey_height,
		)]))
	}
}
