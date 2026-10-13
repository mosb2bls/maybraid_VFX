//! One circular storey shell: wall sweeps + optional floor / ceiling slabs.
//!
//! Openings are resolved in two layers:
//! 1. **Wall sweeps** ([`walls`]) — 15° sectors (AABB-approximated). Hit sectors
//!    omit the opening's \(Y\) span; remaining footer / header bands use
//!    [`Partition::slice_arc`] scaled in \(Y\) to the band height.
//! 2. **Floor / ceiling** ([`slabs`]) — slab-cutting openings that hit a Solid
//!    slab contribute a centered hole sized from the intersection scale (or
//!    remove the slab entirely).
//!
//! Floor / ceiling [`ArcFloorSlab`] values are only [`None`](ArcFloorSlab::None) /
//! [`Solid`](ArcFloorSlab::Solid). They are mainly for towering ownership; openings
//! still map whether or not a slab is present, and can override a Solid slab.
//!
//! Ring locus follows [`building_components::arc_ring_dir`] (kit on \(+X\)).

mod openings;
mod ring;
mod slabs;
mod walls;

#[cfg(test)]
mod tests;

use bevy_math::{Vec2, Vec3};
use building_components::floors::FloorNode;
use building_components::partitions::{PartitionNode, PartitionStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::openings::{MappedOpenings, Openings};

use ring::SEG_DEG;

/// Horizontal storey slab presentation for towering ownership.
///
/// Openings may still cut a Solid slab (Layer 2). Prefer openings for voids;
/// keep these variants simple so stack ownership stays obvious.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcFloorSlab {
	/// Omit the slab entirely (no floor/ceiling geometry).
	None,
	/// Squared floor fill (inscribed-square caps + solid inscribed square).
	/// Openings that intersect this slab may dig a centered hole or remove it.
	Solid,
}

impl Default for ArcFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for an [`ArcFloor`] shell.
///
/// Prefer fluent construction (`ArcFloorParams::new(…).floor(…).build()`), or
/// pass a filled struct to [`ArcFloor::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct ArcFloorParams {
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// World-space void plan applied at construct time.
	///
	/// **Walls (Layer 1):** each opening’s AABB is tested against the 15° sector
	/// AABBs around the ring ([`arc_ring_dir`](building_components::arc_ring_dir)).
	/// Intersecting sectors omit that opening’s \(Y\) span; remaining footer /
	/// header bands become vertically scaled slice-arc strips.
	/// Connectable labels (`Passage` / `Aperture` / `Shaft`) that cut at least
	/// one sector are retained and mapped to an outward contact quad on the
	/// hit span.
	///
	/// **Slabs (Layer 2):** only [`OpeningLabel::cuts_slab`](crate::openings::OpeningLabel::cuts_slab)
	/// labels (`Boundary` / `Exclusion` / `Shaft` / `Custom`) affect Solid floor
	/// or ceiling — by intersection **scale** (centered inscribed hole, or full
	/// slab removal when scale ≥ \(1.4\cdot R\)). Passage / aperture do not cut
	/// slabs.
	pub openings: Openings,
	/// Towering ownership hint; openings may override a Solid slab.
	pub floor: ArcFloorSlab,
	/// Towering ownership hint; openings may override a Solid slab.
	pub ceiling: ArcFloorSlab,
	pub style: PartitionStyle,
}

impl Default for ArcFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			openings: Openings::new(),
			floor: ArcFloorSlab::None,
			ceiling: ArcFloorSlab::None,
			style: PartitionStyle::RoughStonework,
		}
	}
}

impl ArcFloorParams {
	pub fn new(center_xz: Vec3, radius: f32, storey_height: f32) -> Self {
		Self { center_xz, radius, storey_height, ..Self::default() }
	}

	pub fn floor(mut self, floor: ArcFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: ArcFloorSlab) -> Self {
		self.ceiling = ceiling;
		self
	}

	pub fn style(mut self, style: PartitionStyle) -> Self {
		self.style = style;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	pub fn build(self) -> ArcFloor {
		ArcFloor::from_params(self)
	}
}

/// One circular storey: wall partitions + optional floor / ceiling.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcFloor {
	params: ArcFloorParams,
	wall_partitions: Vec<PartitionNode>,
	floor_nodes: Vec<FloorNode>,
	ceiling_nodes: Vec<FloorNode>,
	/// Connectable openings that participated in wall mapping.
	openings: Openings,
	/// Contact geometry for mapped openings.
	mapped: MappedOpenings,
}

impl ArcFloor {
	pub fn new(params: ArcFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: ArcFloorParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params = ArcFloorParams { center_xz, radius, storey_height, ..params };

		let (sectors, wall_partitions) = params.resolve_wall_sweeps();
		let (openings, mapped) = params.map_connectable_openings(&sectors);
		let floor_nodes = params.resolve_floor_nodes();
		let ceiling_nodes = params.resolve_ceiling_nodes();

		Self { params, wall_partitions, floor_nodes, ceiling_nodes, openings, mapped }
	}

	pub fn params(&self) -> &ArcFloorParams {
		&self.params
	}

	pub fn wall_partitions(&self) -> &[PartitionNode] {
		&self.wall_partitions
	}

	pub fn floor_nodes(&self) -> &[FloorNode] {
		&self.floor_nodes
	}

	pub fn ceiling_nodes(&self) -> &[FloorNode] {
		&self.ceiling_nodes
	}

	/// One kit segment in unit \(t\) (15° / 360°).
	pub fn segment_t(&self) -> f32 {
		SEG_DEG / 360.0
	}

	/// Outward unit direction in XZ at normalized sweep parameter \(t\)
	/// ([`arc_ring_dir`](building_components::arc_ring_dir)).
	pub fn ring_dir_at(&self, t: f32) -> Vec2 {
		self.params.ring_dir_at(t)
	}

	/// World point on the ring exterior at \(t\) (floor elevation).
	pub fn ring_point_at(&self, t: f32) -> Vec3 {
		let dir = self.ring_dir_at(t);
		let c = self.params.center_xz;
		let r = self.params.radius;
		Vec3::new(c.x + dir.x * r, c.y, c.z + dir.y * r)
	}
}

impl BuildingComponents for ArcFloor {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
			Layers::from_free(self.wall_partitions.clone())
		} else {
			Layers::new()
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		// Same band as the ring wall. High-only left a clear-color hole in the
		// deck at Medium, and the inscribed-square caps only appeared up close.
		if !matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
			return Layers::new();
		}
		let mut nodes = self.floor_nodes.clone();
		nodes.extend(self.ceiling_nodes.iter().cloned());
		Layers::from_free(nodes)
	}
}
