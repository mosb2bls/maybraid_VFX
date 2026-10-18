//! Corner keep: a circular or trazaloid shell plus storey-to-storey stairwells.

use building_components::floors::FloorGeometry;
use building_components::panels::PanelStyle;
use building_components::partitions::PartitionStyle;
use building_components::{
	BuildingComponents, BuildingStructuralLodProbe, FloorNode, JointNode, Layers, MassingVolume,
	PanelNode, PartitionNode, Placement,
};
use buildings::{
	ArcFloor, ArcFloorSlab, ArcTower, ArcTowerParams, ConnectingStairwell, OpeningId, OpeningLabel,
	Openings, StairwellKind, Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab, WellAabb,
	WellSide,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;

use bevy_math::{Vec2, Vec3};

use crate::finish::paint_stairwell;

pub const TOWER_STOREY_HEIGHT: f32 = 3.2 * 1.5;
const KEEP_TREAD_FILL: f32 = 0.55;

/// Shell plus the wells that climb it.
#[derive(Debug, Clone, PartialEq)]
pub struct Keep<S> {
	pub shell: S,
	pub stairwells: Vec<ConnectingStairwell>,
}

impl<S> Keep<S> {
	pub fn new(shell: S, stairwells: Vec<ConnectingStairwell>) -> Self {
		Self { shell, stairwells }
	}
}

/// Circular keep: stacked [`ArcTower`] storeys with an optional wall shader stamp.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularTower {
	tower: ArcTower,
	wall_material: Option<MaterialRef>,
}

impl CircularTower {
	pub fn inner(&self) -> &ArcTower {
		&self.tower
	}

	pub fn storey_count(&self) -> usize {
		self.tower.params().floor_count as usize
	}

	pub fn center_xz(&self) -> Vec3 {
		self.tower.params().center_xz
	}

	pub fn radius(&self) -> f32 {
		self.tower.params().radius
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}
}

impl BuildingComponents for CircularTower {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = self.tower.partition_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		let mut out = self.tower.floor_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let mut probe = self.tower.structural_lod()?;
		if let Some(material) = &self.wall_material {
			for volume in &mut probe.volumes {
				volume.material = Some(material.clone());
			}
		}
		Some(probe)
	}
}

/// Vertically stacked trazaloid storeys sharing one tapering silhouette.
#[derive(Debug, Clone, PartialEq)]
pub struct TrazaloidTower {
	storeys: Vec<Trazaloid>,
	wall_material: Option<MaterialRef>,
	well_half: f32,
}

impl TrazaloidTower {
	pub fn storeys(&self) -> &[Trazaloid] {
		&self.storeys
	}

	pub fn storey_count(&self) -> usize {
		self.storeys.len()
	}

	pub fn origin(&self) -> Vec3 {
		self.storeys.first().map(|s| s.params().origin).unwrap_or(Vec3::ZERO)
	}

	pub fn base_half_extent(&self) -> f32 {
		self.storeys
			.first()
			.map(|s| s.params().footprint.x.max(s.params().footprint.y) * 0.5)
			.unwrap_or(0.0)
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}
}

impl BuildingComponents for TrazaloidTower {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.panel_nodes_for_level(level));
		}
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for storey in &self.storeys {
			out.extend(storey.joint_nodes_for_level(level));
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		if !matches!(level, LodSceneLevel::High) {
			return Layers::new();
		}
		let mut out = Layers::new();
		for (i, storey) in self.storeys.iter().enumerate() {
			let params = storey.params();
			let hole = (i > 0).then_some(self.well_half);
			for node in keep_rect_floor(params.origin, params.footprint.x, params.footprint.y, hole)
			{
				out.push_free(node);
			}
		}
		if let Some(top) = self.storeys.last() {
			let params = top.params();
			let roof = params.origin + Vec3::Y * TOWER_STOREY_HEIGHT;
			for node in keep_rect_floor(roof, params.ridge.x, params.ridge.y, Some(self.well_half))
			{
				out.push_free(node);
			}
		}
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let first = self.storeys.first()?;
		let last = self.storeys.last()?;
		let foot = first.params();
		let ridge = last.params();
		let top =
			ridge.origin.y + ridge.lower_height + ridge.band_vertical_offset + ridge.upper_height;
		let height = (top - foot.origin.y).max(1.0);
		Some(BuildingStructuralLodProbe::from_volumes([MassingVolume::trazaloid(
			foot.origin,
			foot.footprint,
			ridge.ridge,
			height,
		)
		.with_material_opt(self.wall_material.clone())]))
	}
}

/// Circular [`ArcTower`] or stacked [`Trazaloid`] keep at a ring-fort corner.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortKeep {
	Circular(Keep<CircularTower>),
	Trazaloid(Keep<TrazaloidTower>),
}

impl RingFortKeep {
	pub fn storey_count(&self) -> usize {
		match self {
			Self::Circular(keep) => keep.shell.storey_count(),
			Self::Trazaloid(keep) => keep.shell.storey_count(),
		}
	}

	pub fn center_xz(&self) -> Vec3 {
		match self {
			Self::Circular(keep) => keep.shell.center_xz(),
			Self::Trazaloid(keep) => keep.shell.origin(),
		}
	}

	pub fn plan_half_extent(&self) -> f32 {
		match self {
			Self::Circular(keep) => keep.shell.radius(),
			Self::Trazaloid(keep) => keep.shell.base_half_extent(),
		}
	}

	pub fn stairwells(&self) -> &[ConnectingStairwell] {
		match self {
			Self::Circular(keep) => &keep.stairwells,
			Self::Trazaloid(keep) => &keep.stairwells,
		}
	}

	pub fn with_wall_material(self, wall: MaterialRef) -> Self {
		match self {
			Self::Circular(keep) => Self::Circular(Keep {
				shell: keep.shell.with_wall_material(wall.clone()),
				stairwells: stamp_stairs(keep.stairwells, wall),
			}),
			Self::Trazaloid(keep) => Self::Trazaloid(Keep {
				shell: keep.shell.with_wall_material(wall.clone()),
				stairwells: stamp_stairs(keep.stairwells, wall),
			}),
		}
	}

	pub fn circular(origin: Vec3, radius: f32, floors: usize) -> Self {
		let well_half = keep_well_half(radius);
		let shell = build_circular_tower(origin, radius, floors, well_half * 2.0);
		let stairwells = keep_stairwells(origin, well_half, floors, StairwellKind::Circular, false);
		Self::Circular(Keep::new(shell, stairwells))
	}

	pub fn trazaloid(origin: Vec3, foot: f32, floors: usize, corner: (f32, f32)) -> Self {
		let n = floors.max(1) as f32;
		let t_top = (n - 1.0) / n;
		let top_foot = foot + (foot * 0.48 - foot) * t_top;
		let well_half = keep_well_half(top_foot * 0.5);
		let shell = build_trazaloid_tower(origin, foot, floors, corner, well_half);
		let stairwells =
			keep_stairwells(origin, well_half, floors, StairwellKind::Rectangular, true);
		Self::Trazaloid(Keep::new(shell, stairwells))
	}
}

impl BuildingComponents for RingFortKeep {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(keep) => keep.shell.panel_nodes_for_level(level),
		}
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		match self {
			Self::Circular(keep) => keep.shell.partition_nodes_for_level(level),
			Self::Trazaloid(_) => Layers::new(),
		}
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		match self {
			Self::Circular(keep) => keep.shell.floor_nodes_for_level(level),
			Self::Trazaloid(keep) => keep.shell.floor_nodes_for_level(level),
		}
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		match self {
			Self::Circular(_) => Layers::new(),
			Self::Trazaloid(keep) => keep.shell.joint_nodes_for_level(level),
		}
	}
}

fn stamp_stairs(
	stairwells: Vec<ConnectingStairwell>,
	wall: MaterialRef,
) -> Vec<ConnectingStairwell> {
	stairwells
		.into_iter()
		.map(|stair| paint_stairwell(stair, wall.clone()))
		.collect()
}

fn keep_rect_floor(origin: Vec3, width: f32, depth: f32, hole_half: Option<f32>) -> Vec<FloorNode> {
	let width = width.max(1e-4);
	let depth = depth.max(1e-4);
	let cx = origin.x;
	let cz = origin.z;
	let y = origin.y;
	let center = Vec3::new(cx, y, cz);
	let Some(hh) = hole_half.filter(|h| *h > 1e-4) else {
		return vec![rect_floor_slab(center, width, depth)];
	};
	let hx = width * 0.5;
	let hz = depth * 0.5;
	let hh = hh.min(hx * 0.92).min(hz * 0.92);
	let hole_min_x = cx - hh;
	let hole_max_x = cx + hh;
	let hole_min_z = cz - hh;
	let hole_max_z = cz + hh;
	let min_x = cx - hx;
	let max_x = cx + hx;
	let min_z = cz - hz;
	let max_z = cz + hz;
	let mut nodes = Vec::new();
	let south = hole_min_z - min_z;
	let north = max_z - hole_max_z;
	let west = hole_min_x - min_x;
	let east = max_x - hole_max_x;
	if south > 0.15 {
		nodes.push(rect_floor_slab(Vec3::new(cx, y, 0.5 * (min_z + hole_min_z)), width, south));
	}
	if north > 0.15 {
		nodes.push(rect_floor_slab(Vec3::new(cx, y, 0.5 * (hole_max_z + max_z)), width, north));
	}
	if west > 0.15 {
		nodes.push(rect_floor_slab(Vec3::new(0.5 * (min_x + hole_min_x), y, cz), west, hh * 2.0));
	}
	if east > 0.15 {
		nodes.push(rect_floor_slab(Vec3::new(0.5 * (hole_max_x + max_x), y, cz), east, hh * 2.0));
	}
	nodes
}

fn rect_floor_slab(center: Vec3, width_x: f32, depth_z: f32) -> FloorNode {
	let width_x = width_x.max(1e-4);
	let depth_z = depth_z.max(1e-4);
	let origin = Vec3::new(center.x - 0.5 * width_x, center.y, center.z - 0.5 * depth_z);
	FloorNode::rough_stone(
		FloorGeometry::rectangle(),
		Placement::new(origin, 0.0).with_scale(Vec3::new(width_x, 0.2, depth_z)),
	)
}

fn keep_well_half(plan_half: f32) -> f32 {
	(plan_half * 0.32).clamp(1.7, 3.4)
}

fn keep_stairwells(
	origin: Vec3,
	well_half: f32,
	floors: usize,
	kind: StairwellKind,
	onto_roof: bool,
) -> Vec<ConnectingStairwell> {
	let last_well_i = if onto_roof { floors.saturating_sub(1) } else { floors.saturating_sub(2) };
	if floors < 2 && !onto_roof {
		return Vec::new();
	}
	if floors == 0 {
		return Vec::new();
	}
	let mut out = Vec::with_capacity(last_well_i + 1);
	// Same-face walk-on / walk-off (Les Halles / gallery stacked). Rectangular
	// fit hugs four walls; stacked wells share this face so the upper slab
	// is the next run-in, not a dead-end landing on the opposite side.
	for i in 0..=last_well_i {
		let y0 = origin.y + i as f32 * TOWER_STOREY_HEIGHT;
		let y1 = y0 + TOWER_STOREY_HEIGHT;
		let well = WellAabb::from_plan(
			Vec3::new(origin.x - well_half, y0, origin.z - well_half),
			Vec3::new(origin.x + well_half, y1, origin.z + well_half),
			WellSide::PosX,
			WellSide::PosX,
			KEEP_TREAD_FILL,
		);
		out.push(
			ConnectingStairwell::from_well_kind(PanelStyle::RoughStonework, well, kind)
				.with_upper_landing(i == last_well_i)
				.with_shaft_walls(true),
		);
	}
	out
}

fn build_circular_tower(origin: Vec3, radius: f32, floors: usize, hole: f32) -> CircularTower {
	let mut openings = Openings::new();
	for (i, t) in [0.0_f32, 0.25, 0.5, 0.75].into_iter().enumerate() {
		let (id, opening) = ArcFloor::plan_opening_at_t(
			format!("window_{i}"),
			OpeningLabel::Aperture,
			origin,
			radius,
			TOWER_STOREY_HEIGHT,
			t,
		);
		openings.insert(id, opening);
	}
	CircularTower {
		tower: ArcTower::new(ArcTowerParams {
			center_xz: origin,
			radius,
			floor_count: floors as u32,
			storey_height: TOWER_STOREY_HEIGHT,
			openings,
			base_floor: ArcFloorSlab::Solid,
			intermediate_floors: ArcFloorSlab::Solid,
			top_ceiling: ArcFloorSlab::Solid,
			intermediate_floor_hole: hole,
			style: PartitionStyle::RoughStonework,
		}),
		wall_material: None,
	}
}

fn build_trazaloid_tower(
	origin: Vec3,
	foot: f32,
	floors: usize,
	corner: (f32, f32),
	well_half: f32,
) -> TrazaloidTower {
	let n = floors.max(1) as f32;
	let base = Vec2::splat(foot);
	let ridge_top = base * 0.48;
	let inner = inward_sides(corner.0, corner.1);
	let mut storeys = Vec::with_capacity(floors);
	for i in 0..floors {
		let t0 = i as f32 / n;
		let t1 = (i + 1) as f32 / n;
		let footprint = base.lerp(ridge_top, t0);
		let ridge = base.lerp(ridge_top, t1);
		let y = origin.y + i as f32 * TOWER_STOREY_HEIGHT;
		let mut openings = Openings::new();
		if i == 0 {
			for (k, side) in inner.into_iter().enumerate() {
				openings.insert(
					OpeningId::new(format!("gate_{k}")),
					Trazaloid::side_passage_opening(
						side,
						footprint,
						(footprint.x.min(footprint.y) * 0.22).clamp(1.1, 1.8)
							* buildings::DOOR_SIZE_SCALE,
						2.1 * buildings::DOOR_SIZE_SCALE,
					),
				);
			}
		}
		storeys.push(
			TrazaloidParams {
				origin: Vec3::new(origin.x, y, origin.z),
				footprint,
				ridge,
				lower_height: TOWER_STOREY_HEIGHT * 0.70,
				upper_height: TOWER_STOREY_HEIGHT * 0.20,
				band_vertical_offset: TOWER_STOREY_HEIGHT * 0.10,
				openings,
				floor: TrazaloidSlab::None,
				// Roof is a holed kit deck (see `floor_nodes_for_level`), not a solid lid.
				ceiling: TrazaloidSlab::None,
				style: PanelStyle::RoughStonework,
				..TrazaloidParams::default()
			}
			.build(),
		);
	}
	TrazaloidTower { storeys, wall_material: None, well_half }
}

fn inward_sides(sx: f32, sz: f32) -> [TrazaloidSide; 2] {
	let x_side = if sx > 0.0 { TrazaloidSide::West } else { TrazaloidSide::East };
	let z_side = if sz > 0.0 { TrazaloidSide::South } else { TrazaloidSide::North };
	[x_side, z_side]
}

#[cfg(test)]
mod tests {
	use material_ref::MaterialId;

	use super::*;

	#[test]
	fn trazaloid_keep_owns_floors_with_stair_holes() -> anyhow::Result<()> {
		let origin = Vec3::new(40.0, 12.0, -25.0);
		let keep = RingFortKeep::trazaloid(origin, 16.0, 5, (1.0, 1.0));
		let RingFortKeep::Trazaloid(keep) = keep else {
			anyhow::bail!("expected trazaloid keep");
		};
		let n = keep.shell.storey_count();
		anyhow::ensure!(n == 5, "storey count {n}");
		for (i, storey) in keep.shell.storeys().iter().enumerate() {
			anyhow::ensure!(
				storey.floor().is_none(),
				"storey {i} panel floor would cover kit flooring"
			);
			if i + 1 == n {
				anyhow::ensure!(
					storey.ceiling().is_none(),
					"roof is a holed kit deck, not a solid lid"
				);
			} else {
				anyhow::ensure!(
					storey.ceiling().is_none(),
					"storey {i} should not cover the floor above"
				);
			}
		}
		let floors = keep.shell.floor_nodes_for_level(LodSceneLevel::High);
		let nodes: Vec<_> = floors.flatten();
		anyhow::ensure!(!nodes.is_empty(), "trazaloid keep should emit floor nodes");
		let ground = keep_rect_floor(origin, 16.0, 16.0, None);
		anyhow::ensure!(ground.len() == 1, "ground floor is one slab");
		let intermediate = keep_rect_floor(
			origin + Vec3::Y * TOWER_STOREY_HEIGHT,
			16.0,
			16.0,
			Some(keep.shell.well_half),
		);
		anyhow::ensure!(intermediate.len() == 4, "intermediate floors keep a stair hole");
		anyhow::ensure!(nodes.len() > ground.len(), "upper storeys should add holed floors");
		anyhow::ensure!(
			keep.stairwells.len() == 5,
			"five storeys plus roof → five wells, got {}",
			keep.stairwells.len()
		);
		let roof = keep.stairwells.last().expect("roof well");
		let top = origin.y + 5.0 * TOWER_STOREY_HEIGHT;
		anyhow::ensure!(
			(roof.well().top_y() - top).abs() < 1e-3,
			"last well should land on the roof"
		);
		anyhow::ensure!(roof.upper_landing().is_some(), "roof pad");
		Ok(())
	}

	#[test]
	fn keep_wells_walk_on_and_off_the_same_face() -> anyhow::Result<()> {
		let origin = Vec3::ZERO;
		let wells = keep_stairwells(origin, 2.4, 4, StairwellKind::Rectangular, false);
		anyhow::ensure!(wells.len() == 3, "four floors → three wells");
		for (i, well) in wells.iter().enumerate() {
			let aabb = well.well();
			anyhow::ensure!(
				aabb.walk_on == WellSide::PosX && aabb.walk_off == WellSide::PosX,
				"well {i} should share one face"
			);
			anyhow::ensure!(
				well.shaft_walls().len() == 3,
				"same-face well should wall the three closed sides"
			);
			if i + 1 < wells.len() {
				anyhow::ensure!(
					aabb.walk_off == wells[i + 1].well().walk_on,
					"stacked wells should share the walk-off face"
				);
				anyhow::ensure!(well.upper_landing().is_none(), "shared-face omit");
			} else {
				anyhow::ensure!(well.upper_landing().is_some(), "last-storey pad");
			}
		}

		let last = wells.last().expect("wells");
		anyhow::ensure!(
			last.stairs().len() == 4,
			"same-face circuit should hug four walls, got {}",
			last.stairs().len()
		);
		anyhow::ensure!(
			last.mid_landings().len() == 3,
			"four flights need three corner pads, got {}",
			last.mid_landings().len()
		);

		let circular = keep_stairwells(origin, 2.4, 3, StairwellKind::Circular, false);
		anyhow::ensure!(circular.len() == 2);
		for well in &circular {
			let aabb = well.well();
			anyhow::ensure!(aabb.walk_on == WellSide::PosX && aabb.walk_off == WellSide::PosX);
			anyhow::ensure!(
				well.shaft_walls().len() == 3,
				"circular keep should wall the three closed faces"
			);
		}
		Ok(())
	}

	#[test]
	fn keep_floors_and_stairs_take_the_wall_look() -> anyhow::Result<()> {
		use building_components::BuildingComponents;

		let wall = MaterialRef::named("stucco");
		let keep =
			RingFortKeep::trazaloid(Vec3::ZERO, 16.0, 4, (1.0, 1.0)).with_wall_material(wall);
		let floors = keep.floor_nodes_for_level(LodSceneLevel::High).flatten();
		anyhow::ensure!(!floors.is_empty(), "trazaloid keep should emit floors");
		anyhow::ensure!(
			floors.iter().all(|n| {
				matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
			}),
			"keep floors should carry the wall look"
		);
		let stairs = keep.stairwells();
		anyhow::ensure!(!stairs.is_empty());
		for well in stairs {
			anyhow::ensure!(well.shaft_walls().len() == 3);
			anyhow::ensure!(
				matches!(well.surface_material().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco"),
				"shaft walls stay with the surround"
			);
			let nodes = well.stair_nodes_for_level(LodSceneLevel::High).flatten();
			anyhow::ensure!(!nodes.is_empty());
			anyhow::ensure!(
				nodes.iter().all(|n| {
					matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "wood")
				}),
				"stone surrounds should get wood treads"
			);
			let panels = well.panel_nodes_for_level(LodSceneLevel::High).flatten();
			anyhow::ensure!(
				panels.iter().any(|n| {
					matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "wood")
				}),
				"stone surrounds should get wood landings"
			);
			anyhow::ensure!(
				panels.iter().any(|n| {
					matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
				}),
				"shaft walls stay with the surround"
			);
		}

		let timber = MaterialRef::named("wood");
		let wooden =
			RingFortKeep::trazaloid(Vec3::ZERO, 16.0, 3, (1.0, 1.0)).with_wall_material(timber);
		for well in wooden.stairwells() {
			anyhow::ensure!(
				matches!(well.stair_material().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "furniture_marble"),
				"wood surrounds should get marble treads"
			);
			anyhow::ensure!(
				well.panel_nodes_for_level(LodSceneLevel::High).flatten().iter().any(|n| {
					matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "furniture_marble")
				}),
				"wood surrounds should get marble landings"
			);
		}
		Ok(())
	}
}
