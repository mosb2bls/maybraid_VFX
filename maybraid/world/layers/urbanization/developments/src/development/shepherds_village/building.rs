//! Small shepherd dwellings and their per-building host records.
//!
//! A house is a complete I-frame livable stack (one or two storeys), its
//! connecting stair, and a thatch roof complex. A hut is one rectangular shell
//! and one thatch roof. Each value is deliberately a complete
//! [`BuildingComponents`] host so a village can spawn one LOD host per building.

use std::sync::Arc;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::panels::PanelStyle;
use building_components::{
	BuildingComponents, BuildingStructuralLodProbe, DoorNode, FloorNode, FurnitureNode, JointNode,
	LabelNode, Layers, MassingVolume, PanelNode, PartitionNode, RoofNode, StairNode,
};
use buildings::{
	Confines, ConnectingStairwell, EndCap, FillableRegions, Fit, FitError, IFloor,
	ILivableFloorPlan, ILivableParameterized, ILivableStorey, Opening, OpeningId, OpeningLabel,
	Openings, Overhang, RectFloor, RectFloorParams, RectFloorSide, RectFloorSlab,
	RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams, StairwellKind, WellAabb,
	MIN_HALL_WIDTH, MIN_POCKET_SHAFT_SIDE,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::placed::{BuildingFootprint, PlacedBuilding};

pub const HOUSE_MIN_FOOTPRINT: f32 = 12.0;
pub const HOUSE_MAX_FOOTPRINT: f32 = 24.0;
pub const HUT_MIN_FOOTPRINT: f32 = 4.0;
pub const HUT_MAX_FOOTPRINT: f32 = 8.0;
pub const HOUSE_STOREY_HEIGHT: f32 = 3.0 * 1.3;
pub const HUT_HEIGHT: f32 = 2.8;
/// House corridors and stair shafts run wider than the shared I-frame floors.
pub const HOUSE_MIN_HALL_WIDTH: f32 = MIN_HALL_WIDTH * 1.2;
pub const HOUSE_MIN_SHAFT_SIDE: f32 = MIN_POCKET_SHAFT_SIDE * 2.0;

const SALT_STOREYS: f32 = 71.0;
const SALT_WALL_STYLE: f32 = 73.0;
const SALT_ROOF_CAP: f32 = 79.0;
const TWO_STOREY_THRESHOLD: f32 = 0.70;
const STAIR_TREAD_FILL: f32 = 0.82;

/// Shader materials applied independently from the panel kit styles.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsFinish {
	pub wall: MaterialRef,
	pub roof: MaterialRef,
}

/// Complete one- or two-storey I-frame house.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsHouse {
	pub bounds: Aabb3d,
	pub storeys: Vec<ILivableStorey>,
	pub stairwell: Option<ConnectingStairwell>,
	pub roof: RectangularPitchedRoofComplex,
	pub wall_style: PanelStyle,
	pub finish: Option<ShepherdsFinish>,
}

impl ShepherdsHouse {
	pub fn storey_count(&self) -> usize {
		self.storeys.len()
	}

	pub fn with_finish(mut self, finish: ShepherdsFinish) -> Self {
		self.finish = Some(finish);
		self
	}

	/// Exact orthogonal I-frame pieces used to build matching pad nodes.
	pub fn footprint_rects(&self) -> Vec<Aabb2d> {
		self.storeys
			.first()
			.map(|s| s.floor_plan.primary_rects.iter().map(|r| r.to_aabb2()).collect())
			.unwrap_or_default()
	}
}

impl BuildingFootprint for ShepherdsHouse {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		ShepherdsHouse::footprint_rects(self)
	}
}

impl Fit for ShepherdsHouse {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let extent = Vec3::from(confines.bounds.max - confines.bounds.min);
		if extent.x < HOUSE_MIN_FOOTPRINT || extent.z < HOUSE_MIN_FOOTPRINT {
			return Err(FitError::TooSmall { reason: "shepherds_house_footprint" });
		}

		let cfg = NoiseConfig::new(noise);
		let center = confines.center();
		let sampled_two =
			cfg.sample_unit_4d(center.x, center.y, center.z, SALT_STOREYS) >= TWO_STOREY_THRESHOLD;
		let storey_count = if sampled_two { 2 } else { 1 };
		let needed_height = storey_count as f32 * HOUSE_STOREY_HEIGHT;
		if extent.y + 1e-3 < needed_height {
			return Err(FitError::TooSmall { reason: "shepherds_house_height" });
		}

		let wall_style = if cfg.sample_unit_4d(center.x, center.y, center.z, SALT_WALL_STYLE) < 0.5
		{
			PanelStyle::RibAndPlank
		} else {
			PanelStyle::RoughStonework
		};

		let y0 = confines.bounds.min.y;
		let base_bounds = Aabb3d::from_min_max(
			Vec3::new(confines.bounds.min.x, y0, confines.bounds.min.z),
			Vec3::new(confines.bounds.max.x, y0 + HOUSE_STOREY_HEIGHT, confines.bounds.max.z),
		);
		let base_empty = Confines::new(base_bounds, confines.roll, Openings::new());
		let mut params = ILivableParameterized::sample(&base_empty, noise)?
			.with_minimums(HOUSE_MIN_HALL_WIDTH, HOUSE_MIN_SHAFT_SIDE);
		// I-frame family only: preserve sampled I/T/L/Z variants, but reject the
		// sampler's small plain-stem tail.
		if !params.has_top_flange() && !params.has_bottom_flange() {
			params.top_left_share = Some(0.45);
			params.top_right_share = Some(0.45);
		}

		let provisional = ILivableFloorPlan::from_parameterized(params.clone(), &base_empty)?.0;
		let door = exterior_door(&provisional)
			.ok_or(FitError::TooSmall { reason: "shepherds_house_door_edge" })?;

		let shaft_openings = if storey_count == 2 {
			roomiest_shaft(
				ILivableFloorPlan::shaft_requests_for_primary_rects(&params, &base_empty),
				&provisional,
			)
		} else {
			Openings::new()
		};
		let mut base_openings = shaft_openings.clone();
		base_openings.insert(OpeningId::scoped("shepherds_house", "outer_door", "entry"), door);

		let mut storeys = Vec::with_capacity(storey_count);
		for i in 0..storey_count {
			let floor_y = y0 + i as f32 * HOUSE_STOREY_HEIGHT;
			let openings = if i == 0 {
				rebase_openings_y(&base_openings, floor_y - y0)
			} else {
				rebase_openings_y(&shaft_openings, floor_y - y0)
			};
			let floor_bounds = Aabb3d::from_min_max(
				Vec3::new(confines.bounds.min.x, floor_y, confines.bounds.min.z),
				Vec3::new(
					confines.bounds.max.x,
					floor_y + HOUSE_STOREY_HEIGHT,
					confines.bounds.max.z,
				),
			);
			let floor_confines = Confines::new(floor_bounds, confines.roll, openings);
			let ceiling = if i + 1 == storey_count {
				buildings::IFloorSlab::Solid
			} else {
				buildings::IFloorSlab::None
			};
			let (plan, _) = ILivableFloorPlan::from_parameterized_with_ceiling(
				params.clone(),
				&floor_confines,
				ceiling,
			)?;
			let mut floor_noise = noise;
			floor_noise.seed = floor_noise.seed.wrapping_add(i as i32 * 97);
			storeys.push(ILivableStorey::from_floor_plan(plan, floor_noise)?.0);
		}

		let stairwell = if storey_count == 2 {
			let shaft = storeys
				.first()
				.and_then(|s| s.floor_plan.shaft_bounds.first())
				.copied()
				.ok_or(FitError::TooSmall { reason: "shepherds_house_stair" })?;
			let side = side_toward(center, shaft);
			let well = WellAabb::from_plan(
				Vec3::from(shaft.min),
				Vec3::from(shaft.max),
				side,
				side,
				STAIR_TREAD_FILL,
			);
			Some(
				ConnectingStairwell::from_well_kind(wall_style, well, StairwellKind::Rectangular)
					.with_upper_landing(true),
			)
		} else {
			None
		};

		let top = storeys.last().ok_or(FitError::TooSmall { reason: "shepherds_house_storeys" })?;
		let eave_y = y0 + needed_height;
		let volumes = top
			.floor_plan
			.primary_rects
			.iter()
			.map(|r| {
				let rise = (r.width().min(r.depth()) * 0.42).clamp(1.6, 4.2);
				Aabb3d::from_min_max(
					Vec3::new(r.min_x, eave_y, r.min_z),
					Vec3::new(r.max_x, eave_y + rise, r.max_z),
				)
			})
			.collect();
		let end_cap = sampled_end_cap(&cfg, center);
		let roof = RectangularPitchedRoofComplexParams::new(volumes)
			.overhang(Overhang::Fixed(0.55))
			.end_cap(end_cap)
			.style(PanelStyle::ShepherdsThatch)
			.build();

		Ok((
			Self { bounds: confines.bounds, storeys, stairwell, roof, wall_style, finish: None },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

/// Complete one-room rectangular hut.
#[derive(Debug, Clone, PartialEq)]
pub struct ShepherdsHut {
	pub bounds: Aabb3d,
	pub shell: RectFloor,
	pub roof: RectangularPitchedRoofComplex,
	pub wall_style: PanelStyle,
	pub finish: Option<ShepherdsFinish>,
}

impl ShepherdsHut {
	pub fn with_finish(mut self, finish: ShepherdsFinish) -> Self {
		self.finish = Some(finish);
		self
	}

	pub fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingFootprint for ShepherdsHut {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		ShepherdsHut::footprint_rects(self)
	}
}

impl Fit for ShepherdsHut {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let extent = Vec3::from(confines.bounds.max - confines.bounds.min);
		if extent.x < HUT_MIN_FOOTPRINT || extent.z < HUT_MIN_FOOTPRINT {
			return Err(FitError::TooSmall { reason: "shepherds_hut_footprint" });
		}
		if extent.y + 1e-3 < HUT_HEIGHT {
			return Err(FitError::TooSmall { reason: "shepherds_hut_height" });
		}

		let center = confines.center();
		let floor_center = Vec3::new(center.x, confines.bounds.min.y, center.z);
		let footprint = Vec2::new(extent.x, extent.z);
		let cfg = NoiseConfig::new(noise);
		let wall_style = if cfg.sample_unit_4d(center.x, center.y, center.z, SALT_WALL_STYLE) < 0.5
		{
			PanelStyle::RibAndPlank
		} else {
			PanelStyle::RoughStonework
		};
		let openings = Openings::new()
			.with(
				OpeningId::scoped("shepherds_hut", "outer_door", "south"),
				RectFloor::side_passage_opening(
					RectFloorSide::South,
					floor_center,
					footprint,
					1.0,
					2.05,
				),
			)
			.with(
				OpeningId::scoped("shepherds_hut", "window", "east"),
				RectFloor::side_aperture_opening(
					RectFloorSide::East,
					floor_center,
					footprint,
					1.0,
					0.9,
					0.95,
				),
			)
			.with(
				OpeningId::scoped("shepherds_hut", "window", "west"),
				RectFloor::side_aperture_opening(
					RectFloorSide::West,
					floor_center,
					footprint,
					1.0,
					0.9,
					0.95,
				),
			);
		let shell = RectFloorParams::new(floor_center, footprint, HUT_HEIGHT)
			.floor(RectFloorSlab::Solid)
			.style(wall_style)
			.openings(openings)
			.build();

		let eave_y = confines.bounds.min.y + HUT_HEIGHT;
		let rise = (footprint.x.min(footprint.y) * 0.45).clamp(1.2, 3.0);
		let volume = Aabb3d::from_min_max(
			Vec3::new(confines.bounds.min.x, eave_y, confines.bounds.min.z),
			Vec3::new(confines.bounds.max.x, eave_y + rise, confines.bounds.max.z),
		);
		let roof = RectangularPitchedRoofComplexParams::new(vec![volume])
			.overhang(Overhang::Fixed(0.45))
			.end_cap(sampled_end_cap(&cfg, center))
			.style(PanelStyle::ShepherdsThatch)
			.build();

		Ok((
			Self { bounds: confines.bounds, shell, roof, wall_style, finish: None },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

/// One complete building in a village.
#[derive(Debug, Clone, PartialEq)]
pub enum ShepherdsBuilding {
	House(Arc<ShepherdsHouse>),
	Hut(Arc<ShepherdsHut>),
}

impl BuildingFootprint for ShepherdsBuilding {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		match self {
			Self::House(house) => house.footprint_rects(),
			Self::Hut(hut) => hut.footprint_rects(),
		}
	}
}

/// Complete shepherd building plus its independent pose.
pub type ShepherdsVillageBuilding = PlacedBuilding<ShepherdsBuilding>;

macro_rules! house_layers {
	($self:expr, $level:expr, $method:ident) => {{
		let mut out = Layers::new();
		for storey in &$self.storeys {
			out.extend(storey.$method($level));
		}
		if let Some(stairwell) = &$self.stairwell {
			out.extend(stairwell.$method($level));
		}
		out.extend($self.roof.$method($level));
		out
	}};
}

impl BuildingComponents for ShepherdsHouse {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut walls = Layers::new();
		for storey in &self.storeys {
			walls.extend(storey.panel_nodes_for_level(level));
		}
		if let Some(stairwell) = &self.stairwell {
			walls.extend(stairwell.panel_nodes_for_level(level));
		}
		let mut roof = self.roof.panel_nodes_for_level(level);
		if let Some(finish) = &self.finish {
			walls = walls.with_material(finish.wall.clone());
			roof = roof.with_material(finish.roof.clone());
		}
		walls.extend(roof);
		walls
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		house_layers!(self, level, partition_nodes_for_level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		house_layers!(self, level, floor_nodes_for_level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		house_layers!(self, level, roof_nodes_for_level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		house_layers!(self, level, stair_nodes_for_level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		house_layers!(self, level, door_nodes_for_level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		house_layers!(self, level, joint_nodes_for_level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		house_layers!(self, level, furniture_nodes_for_level)
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		house_layers!(self, level, furniture_usage_nodes_for_level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		house_layers!(self, level, label_nodes_for_level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let y0 = self.bounds.min.y;
		let wall_h = (self.storeys.len() as f32 * HOUSE_STOREY_HEIGHT).max(HOUSE_STOREY_HEIGHT);
		let wall = self.finish.as_ref().map(|f| f.wall.clone());
		let roof = self.finish.as_ref().map(|f| f.roof.clone());
		let mut volumes: Vec<_> = self
			.footprint_rects()
			.into_iter()
			.map(|xz| MassingVolume::cuboid(xz, y0, wall_h).with_material_opt(wall.clone()))
			.collect();
		if let Some(probe) = self.roof.structural_lod() {
			volumes.extend(probe.volumes.into_iter().map(|v| v.with_material_opt(roof.clone())));
		}
		if volumes.is_empty() {
			None
		} else {
			Some(BuildingStructuralLodProbe::from_volumes(volumes))
		}
	}
}

macro_rules! hut_layers {
	($self:expr, $level:expr, $method:ident) => {{
		let mut out = $self.shell.$method($level);
		out.extend($self.roof.$method($level));
		out
	}};
}

impl BuildingComponents for ShepherdsHut {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut walls = self.shell.panel_nodes_for_level(level);
		let mut roof = self.roof.panel_nodes_for_level(level);
		if let Some(finish) = &self.finish {
			walls = walls.with_material(finish.wall.clone());
			roof = roof.with_material(finish.roof.clone());
		}
		walls.extend(roof);
		walls
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		hut_layers!(self, level, partition_nodes_for_level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		hut_layers!(self, level, floor_nodes_for_level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		hut_layers!(self, level, roof_nodes_for_level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		hut_layers!(self, level, stair_nodes_for_level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		hut_layers!(self, level, door_nodes_for_level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		hut_layers!(self, level, joint_nodes_for_level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		hut_layers!(self, level, furniture_nodes_for_level)
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		hut_layers!(self, level, furniture_usage_nodes_for_level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		hut_layers!(self, level, label_nodes_for_level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let wall = self.finish.as_ref().map(|f| f.wall.clone());
		let roof = self.finish.as_ref().map(|f| f.roof.clone());
		let mut volumes = self.shell.structural_lod().map(|p| p.volumes).unwrap_or_default();
		for volume in &mut volumes {
			volume.material = wall.clone();
		}
		if let Some(probe) = self.roof.structural_lod() {
			volumes.extend(probe.volumes.into_iter().map(|v| v.with_material_opt(roof.clone())));
		}
		if volumes.is_empty() {
			None
		} else {
			Some(BuildingStructuralLodProbe::from_volumes(volumes))
		}
	}
}

/// Keep the one shaft request whose host rect has the longest short side, so
/// the stair lands in the wing with room for [`HOUSE_MIN_SHAFT_SIDE`].
fn roomiest_shaft(openings: Openings, plan: &ILivableFloorPlan) -> Openings {
	let room = |opening: &Opening| {
		let center = (Vec3::from(opening.bounds.min) + Vec3::from(opening.bounds.max)) * 0.5;
		plan.primary_rects
			.iter()
			.filter(|r| (r.min_x..=r.max_x).contains(&center.x))
			.filter(|r| (r.min_z..=r.max_z).contains(&center.z))
			.map(|r| r.width().min(r.depth()))
			.fold(0.0_f32, f32::max)
	};
	let Some((id, opening)) = openings.iter().max_by(|(a_id, a), (b_id, b)| {
		room(a).total_cmp(&room(b)).then_with(|| b_id.as_str().cmp(a_id.as_str()))
	}) else {
		return Openings::new();
	};
	Openings::new().with(id.clone(), opening.clone())
}

fn exterior_door(plan: &ILivableFloorPlan) -> Option<Opening> {
	plan.shell
		.edges()
		.iter()
		.copied()
		.map(|edge| {
			let door = IFloor::edge_passage_opening(
				edge,
				1.1 * buildings::DOOR_SIZE_SCALE,
				2.15 * buildings::DOOR_SIZE_SCALE,
			);
			let overlaps = plan
				.openings
				.iter()
				.filter(|(_, opening)| matches!(opening.label, OpeningLabel::Aperture))
				.filter(|(_, opening)| aabb_intersects(door.bounds, opening.bounds))
				.count();
			(edge.length(), overlaps, door)
		})
		.min_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.total_cmp(&a.0)))
		.map(|(_, _, door)| door)
}

fn aabb_intersects(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x
		&& b.min.x <= a.max.x
		&& a.min.y <= b.max.y
		&& b.min.y <= a.max.y
		&& a.min.z <= b.max.z
		&& b.min.z <= a.max.z
}

fn rebase_openings_y(openings: &Openings, dy: f32) -> Openings {
	let mut out = Openings::new();
	for (id, opening) in openings.iter() {
		let mut moved = opening.clone();
		moved.bounds = Aabb3d::from_min_max(
			Vec3::from(moved.bounds.min) + Vec3::Y * dy,
			Vec3::from(moved.bounds.max) + Vec3::Y * dy,
		);
		out.insert(id.clone(), moved);
	}
	out
}

fn side_toward(center: Vec3, shaft: Aabb3d) -> buildings::WellSide {
	let mid = Vec3::from((shaft.min + shaft.max) * 0.5);
	let toward = Vec2::new(center.x - mid.x, center.z - mid.z);
	if toward.x.abs() >= toward.y.abs() {
		if toward.x >= 0.0 {
			buildings::WellSide::PosX
		} else {
			buildings::WellSide::NegX
		}
	} else if toward.y >= 0.0 {
		buildings::WellSide::PosZ
	} else {
		buildings::WellSide::NegZ
	}
}

fn sampled_end_cap(cfg: &NoiseConfig, center: Vec3) -> EndCap {
	if cfg.sample_unit_4d(center.x, center.y, center.z, SALT_ROOF_CAP) < 0.5 {
		EndCap::Hip
	} else {
		EndCap::Gable { ridge: Overhang::Fixed(0.4), eave: Overhang::Fixed(0.4) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn confines(footprint: Vec2) -> Confines {
		Confines::new(
			Aabb3d::from_min_max(
				Vec3::ZERO,
				Vec3::new(footprint.x, 2.0 * HOUSE_STOREY_HEIGHT, footprint.y),
			),
			0.0,
			Openings::new(),
		)
	}

	#[test]
	fn house_is_never_a_plain_stem() {
		let mut one_storey = 0;
		let mut two_storey = 0;
		for seed in 0..32 {
			let noise = NoiseParams { seed, ..NoiseParams::default() };
			let house = ShepherdsHouse::fit_to_confines(&confines(Vec2::splat(18.0)), noise)
				.expect("house fit")
				.0;
			let params = &house.storeys[0].floor_plan.parameterized;
			assert!(params.has_top_flange() || params.has_bottom_flange());
			let plan = &house.storeys[0].floor_plan;
			let door = plan
				.openings
				.iter()
				.find(|(id, _)| id.as_str().contains("outer_door"))
				.map(|(_, opening)| opening)
				.expect("house entry");
			assert!(plan
				.openings
				.iter()
				.filter(|(_, opening)| matches!(opening.label, OpeningLabel::Aperture))
				.all(|(_, opening)| !aabb_intersects(door.bounds, opening.bounds)));
			match house.storey_count() {
				1 => {
					one_storey += 1;
					assert!(house.stairwell.is_none());
				}
				2 => {
					two_storey += 1;
					assert!(house.stairwell.is_some());
				}
				n => panic!("unexpected storey count {n}"),
			}
		}
		assert!(one_storey > two_storey, "70/30 sampling should favor one-storey houses");
		assert!(two_storey > 0, "sample should still contain two-storey houses");
	}

	#[test]
	fn houses_keep_wide_halls_tall_storeys_and_roomy_shafts() -> anyhow::Result<()> {
		let mut stairs = 0;
		for foot in [16.0, 20.0, 24.0] {
			for seed in 0..24 {
				let noise = NoiseParams { seed, ..NoiseParams::default() };
				let (house, _) =
					ShepherdsHouse::fit_to_confines(&confines(Vec2::splat(foot)), noise)?;
				let plan = &house.storeys[0].floor_plan;
				anyhow::ensure!(plan.hall_width >= HOUSE_MIN_HALL_WIDTH - 1e-3);
				anyhow::ensure!((plan.storey_height - HOUSE_STOREY_HEIGHT).abs() < 1e-3);
				let Some(shaft) = plan.shaft_bounds.first() else { continue };
				stairs += 1;
				let side = (shaft.max.x - shaft.min.x).min(shaft.max.z - shaft.min.z);
				anyhow::ensure!(
					side >= HOUSE_MIN_SHAFT_SIDE - 1e-3,
					"{foot} m house seed {seed} shaft is only {side:.2} m"
				);
				anyhow::ensure!(
					plan.primary_rects.iter().any(|rect| {
						shaft.min.x >= rect.min_x
							&& shaft.max.x <= rect.max_x
							&& shaft.min.z >= rect.min_z
							&& shaft.max.z <= rect.max_z
					}),
					"{foot} m house seed {seed} shaft leaves its rect"
				);
			}
		}
		anyhow::ensure!(stairs > 0, "the sample should include two-storey houses");
		Ok(())
	}

	#[test]
	fn hut_has_solid_floor_door_and_two_windows() {
		let hut =
			ShepherdsHut::fit_to_confines(&confines(Vec2::new(6.0, 8.0)), NoiseParams::default())
				.expect("hut fit")
				.0;
		assert!(hut.shell.has_floor());
		assert_eq!(hut.shell.params().openings.len(), 3);
	}
}
