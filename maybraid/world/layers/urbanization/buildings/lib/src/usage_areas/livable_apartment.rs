//! Livable apartment: entry → body cluster → RLA per max-rect.
//!
//! Pipeline and apartment-specific policy live in the `layout` module — start
//! there when designing similar multi-cell residential fills. Stages in brief:
//! entry carve, max-rect passage cluster, program slices → RLA, scrap reclaim,
//! soft normalize.

mod entry;
mod layout;
mod program;
mod room;

#[cfg(test)]
mod tests;

pub use room::ApartmentRoom;

use bevy_math::bounding::{Aabb2d, BoundingVolume};
use bevy_math::Vec3;
use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::{LabelNode, LabelStyle};
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, MultiConfines, SpaceKind};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::shells::RectFloor;
use crate::usage_areas::plan_geom::host_xz;

pub(crate) const EPS: f32 = 1e-3;
pub(crate) const SCOPE: &str = "livable_apartment";
/// Provenance for in-apartment partition / room walls (High structural band only).
pub(crate) const INTERNAL_WALLS_LAYER: &str = "internal_walls";

/// One apartment group: envelope + entryway / rectangular livable areas.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartment {
	pub region_id: u32,
	/// Envelope cells that make up this apartment.
	pub cells: MultiConfines,
	/// Packed spaces (entryway, quarters, household closets, open halls).
	pub rooms: Vec<ApartmentRoom>,
	/// Unwalled circulation bands (≥ walk clear); identification only.
	pub walkways: Vec<Aabb2d>,
	/// Partition strips for bedrooms / bathrooms (with connecting passages).
	pub partitions: Vec<ClippedRectangularStrip>,
	/// Max-rect hosts used for layout (gizmos / debug).
	pub max_rects: Vec<Aabb2d>,
	/// Optional envelope shell for the primary / first cell (presentation).
	pub shell: Option<RectFloor>,
}

impl LivableApartment {
	pub fn from_confines(
		region_id: u32,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_multi(
			region_id,
			&MultiConfines::new([FillRegion::new(SpaceKind::InternalSpace, confines.clone())]),
			noise,
		)
	}

	pub fn from_multi(
		region_id: u32,
		cells: &MultiConfines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		layout::fit_from_multi(region_id, cells, noise)
	}

	pub fn primary_confines(&self) -> &Confines {
		&self.cells.parts[0].confines
	}
}

impl Fit for LivableApartment {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(0, confines, noise)
	}
}

impl BuildingComponents for LivableApartment {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.panel_nodes_for_level(level));
		}
		for wall in &self.partitions {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.panel_nodes_for_level(level));
		}
		for room in &self.rooms {
			out.extend_under(INTERNAL_WALLS_LAYER, room.panel_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.joint_nodes_for_level(level));
		}
		for wall in &self.partitions {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.joint_nodes_for_level(level));
		}
		for room in &self.rooms {
			out.extend_under(INTERNAL_WALLS_LAYER, room.joint_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		let name = format!("Livable {}", self.region_id + 1);
		for part in self.cells.iter() {
			let confines = &part.confines;
			let center = Vec3::from(confines.bounds.center());
			let extents =
				Vec3::from(confines.bounds.max - confines.bounds.min).max(Vec3::splat(1e-4));
			out.push_free(LabelNode::rectangle(
				LabelStyle::Blue,
				&name,
				center,
				extents,
				confines.roll,
			));
		}
		for room in &self.rooms {
			out.extend(room.label_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for room in &self.rooms {
			out.extend(room.furniture_nodes_for_level(level));
		}
		out
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		let mut out = Layers::new();
		for room in &self.rooms {
			out.extend(room.furniture_usage_nodes_for_level(level));
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		if self.cells.is_empty() {
			return None;
		}
		Some(BuildingStructuralLodProbe::new(
			self.cells.iter().map(|part| host_xz(&part.confines.bounds)),
		))
	}
}

/// High keeps tagged internals; coarser bands drop [`INTERNAL_WALLS_LAYER`].
fn structural_layers<T>(level: LodSceneLevel, layers: Layers<T>) -> Layers<T> {
	if matches!(level, LodSceneLevel::High) {
		layers
	} else {
		layers.except([INTERNAL_WALLS_LAYER])
	}
}
