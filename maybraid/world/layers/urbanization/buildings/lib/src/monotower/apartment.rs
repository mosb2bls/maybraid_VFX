//! Apartment monotower used by the single-highrise development family.
//!
//! A tower samples one floor plan and one set of shaft slots, then translates
//! that frozen plan through every storey. The three exterior families share the
//! same [`LivableApartments`] allocator and one aligned stair column per shaft.

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use building_components::doors::DoorNode;
use building_components::floors::FloorNode;
use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::partitions::{PartitionNode, PartitionStyle};
use building_components::roofs::RoofNode;
use building_components::stairs::StairNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers, MassingVolume};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::connecting::{ConnectingStairwell, StairwellKind, WellAabb, WellSide};
use crate::fit::{aabb_xz_extent, Confines, FillableRegions, Fit, FitError};
use crate::openings::{MappedOpening, MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::shells::{
	ArcFloor, ArcFloorParams, ArcFloorSlab, IFloorSlab, RectFloor, RectFloorParams, RectFloorSlab,
};
use crate::storeys::i_apartment::{
	IApartmentFloorPlan, IApartmentFullStorey, IApartmentParameterized,
};
use crate::usage_areas::plan_geom::noise_for_cell;
use crate::usage_areas::{
	CardinalFace, LivableApartments, LivableApartmentsOptions, MIN_HALL_WIDTH,
};

const STOREY_HEIGHT: f32 = 3.2;
const MIN_STOREYS: usize = 8;
const PLAN_INSET: f32 = 0.84;
const CIRCULAR_INSET: f32 = 0.42;
const CIRCULAR_INSCRIBED_HALF: f32 = 0.7;
const SHAFT_FRACTION: f32 = 0.1;
const MIN_SHAFT_SIDE: f32 = 2.4 * 1.5;
const MAX_SHAFT_SIDE: f32 = 4.0 * 1.5;
const HIGHRISE_MIN_HALL_WIDTH: f32 = MIN_HALL_WIDTH * 1.2;
const STAIR_TREAD_FILL: f32 = 0.55;
const BRIDGE_PASSAGE_WIDTH: f32 = 2.4;
const BRIDGE_PASSAGE_DEPTH: f32 = 0.7;
const SCOPE: &str = "single_highrise";

/// Exterior floor-plan family for the one `SingleHighrise` development kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SingleHighrisePlan {
	Circular,
	Rectangular,
	IFrame,
}

impl SingleHighrisePlan {
	/// Deterministically combine seed and envelope aspect ratio.
	pub fn select(seed: i32, footprint: Vec2) -> Self {
		let short = footprint.x.min(footprint.y).max(1e-4);
		let aspect = footprint.x.max(footprint.y) / short;
		let aspect_bias = if aspect >= 1.45 {
			1
		} else if aspect >= 1.15 {
			2
		} else {
			0
		};
		match (seed.rem_euclid(3) + aspect_bias) % 3 {
			0 => Self::Circular,
			1 => Self::Rectangular,
			_ => Self::IFrame,
		}
	}
}

/// Stable plan-space location of one vertical shaft column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SingleHighriseShaftSlot {
	pub bounds: Aabb2d,
}

/// Frozen floor-plan parameters replayed by an [`ApartmentMonotower`].
#[derive(Debug, Clone, PartialEq)]
pub enum SingleHighriseFloorPlan {
	Circular { center_xz: Vec2, radius: f32, interior_half: f32 },
	Rectangular { center_xz: Vec2, footprint: Vec2 },
	IFrame { center_xz: Vec2, footprint: Vec2, parameterized: IApartmentParameterized },
}

impl SingleHighriseFloorPlan {
	pub fn kind(&self) -> SingleHighrisePlan {
		match self {
			Self::Circular { .. } => SingleHighrisePlan::Circular,
			Self::Rectangular { .. } => SingleHighrisePlan::Rectangular,
			Self::IFrame { .. } => SingleHighrisePlan::IFrame,
		}
	}

	pub fn footprint_bounds(&self) -> Aabb2d {
		match self {
			Self::Circular { center_xz, radius, .. } => Aabb2d {
				min: *center_xz - Vec2::splat(*radius),
				max: *center_xz + Vec2::splat(*radius),
			},
			Self::Rectangular { center_xz, footprint }
			| Self::IFrame { center_xz, footprint, .. } => {
				Aabb2d { min: *center_xz - *footprint * 0.5, max: *center_xz + *footprint * 0.5 }
			}
		}
	}
}

/// One apartment storey built from the tower's frozen plan.
#[derive(Debug, Clone, PartialEq)]
pub enum SingleHighriseStorey {
	Circular {
		shell: Box<ArcFloor>,
		blocks: Vec<LivableApartments>,
		shaft_bounds: Vec<Aabb3d>,
	},
	Rectangular {
		shell: Box<RectFloor>,
		apartments: Box<LivableApartments>,
		shaft_bounds: Vec<Aabb3d>,
	},
	IFrame(Box<IApartmentFullStorey>),
}

impl SingleHighriseStorey {
	pub fn apartment_count(&self) -> usize {
		self.apartment_blocks().map(|block| block.apartments.len()).sum()
	}

	pub fn room_count(&self) -> usize {
		self.apartment_blocks()
			.flat_map(|block| &block.apartments)
			.map(|apartment| apartment.rooms.len())
			.sum()
	}

	pub fn shaft_bounds(&self) -> Vec<Aabb3d> {
		match self {
			Self::Circular { shaft_bounds, .. } | Self::Rectangular { shaft_bounds, .. } => {
				shaft_bounds.clone()
			}
			Self::IFrame(storey) => storey.floor_plan.shaft_bounds.clone(),
		}
	}

	fn apartment_blocks(&self) -> impl Iterator<Item = &LivableApartments> {
		let blocks: &[LivableApartments] = match self {
			Self::Circular { blocks, .. } => blocks,
			Self::Rectangular { apartments, .. } => std::slice::from_ref(apartments.as_ref()),
			Self::IFrame(storey) => &storey.blocks,
		};
		blocks.iter()
	}

	fn openings(&self) -> &Openings {
		match self {
			Self::Circular { shell, .. } => shell.openings(),
			Self::Rectangular { shell, .. } => shell.openings(),
			Self::IFrame(storey) => &storey.floor_plan.openings,
		}
	}

	fn mapped_opening(&self, id: &OpeningId) -> Option<&MappedOpening> {
		match self {
			Self::Circular { shell, .. } => shell.mapped_opening(id),
			Self::Rectangular { shell, .. } => shell.mapped_opening(id),
			Self::IFrame(storey) => storey.floor_plan.shell.mapped_opening(id),
		}
	}
}

macro_rules! storey_components {
	($method:ident, $node:ty) => {
		fn $method(&self, level: LodSceneLevel) -> Layers<$node> {
			match self {
				Self::Circular { shell, blocks, .. } => {
					let mut out = shell.$method(level);
					for block in blocks {
						out.extend(block.$method(level));
					}
					out
				}
				Self::Rectangular { shell, apartments, .. } => {
					let mut out = shell.$method(level);
					out.extend(apartments.$method(level));
					out
				}
				Self::IFrame(storey) => storey.$method(level),
			}
		}
	};
}

impl BuildingComponents for SingleHighriseStorey {
	storey_components!(panel_nodes_for_level, PanelNode);
	storey_components!(partition_nodes_for_level, PartitionNode);
	storey_components!(floor_nodes_for_level, FloorNode);
	storey_components!(roof_nodes_for_level, RoofNode);
	storey_components!(stair_nodes_for_level, StairNode);
	storey_components!(door_nodes_for_level, DoorNode);
	storey_components!(joint_nodes_for_level, JointNode);
	storey_components!(furniture_nodes_for_level, FurnitureNode);
	storey_components!(furniture_usage_nodes_for_level, building_components::FurnitureUsageNode);
	storey_components!(label_nodes_for_level, LabelNode);
}

/// One frozen plan, repeated apartment storeys, and aligned shaft circulation.
#[derive(Debug, Clone, PartialEq)]
pub struct ApartmentMonotower {
	pub floor_plan: SingleHighriseFloorPlan,
	pub storey_height: f32,
	pub shaft_slots: Vec<SingleHighriseShaftSlot>,
	pub storeys: Vec<SingleHighriseStorey>,
	pub stairwells: Vec<ConnectingStairwell>,
}

impl ApartmentMonotower {
	pub fn storey_count(&self) -> usize {
		self.storeys.len()
	}

	pub fn apartment_count(&self) -> usize {
		self.storeys.iter().map(SingleHighriseStorey::apartment_count).sum()
	}

	pub fn room_count(&self) -> usize {
		self.storeys.iter().map(SingleHighriseStorey::room_count).sum()
	}

	pub fn bridge_passage_id(storey: usize, direction: CardinalFace) -> OpeningId {
		OpeningId::scoped(SCOPE, "bridge", format!("{}_{}", direction_name(direction), storey))
	}

	pub fn bridge_passage_opening(
		&self,
		storey: usize,
		direction: CardinalFace,
	) -> Option<&Opening> {
		let id = Self::bridge_passage_id(storey, direction);
		self.storeys.get(storey)?.openings().get(&id)
	}

	pub fn mapped_bridge_passage(
		&self,
		storey: usize,
		direction: CardinalFace,
	) -> Option<&MappedOpening> {
		let id = Self::bridge_passage_id(storey, direction);
		self.storeys.get(storey)?.mapped_opening(&id)
	}
}

macro_rules! tower_components {
	($method:ident, $node:ty) => {
		fn $method(&self, level: LodSceneLevel) -> Layers<$node> {
			let mut out = Layers::new();
			for storey in &self.storeys {
				out.extend(storey.$method(level));
			}
			for stairwell in &self.stairwells {
				out.extend(stairwell.$method(level));
			}
			out
		}
	};
}

impl BuildingComponents for ApartmentMonotower {
	tower_components!(panel_nodes_for_level, PanelNode);
	tower_components!(partition_nodes_for_level, PartitionNode);
	tower_components!(floor_nodes_for_level, FloorNode);
	tower_components!(roof_nodes_for_level, RoofNode);
	tower_components!(stair_nodes_for_level, StairNode);
	tower_components!(door_nodes_for_level, DoorNode);
	tower_components!(joint_nodes_for_level, JointNode);
	tower_components!(furniture_nodes_for_level, FurnitureNode);
	tower_components!(furniture_usage_nodes_for_level, building_components::FurnitureUsageNode);
	tower_components!(label_nodes_for_level, LabelNode);
}

/// Consolidated apartment high-rise: shell, rooms, shafts, stairs, and roof slab
/// are presented through one building component host.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleHighrise {
	pub bounds: Aabb3d,
	pub tower: ApartmentMonotower,
	wall_material: Option<MaterialRef>,
}

impl SingleHighrise {
	pub fn plan(&self) -> SingleHighrisePlan {
		self.tower.floor_plan.kind()
	}

	pub fn storey_count(&self) -> usize {
		self.tower.storey_count()
	}

	pub fn apartment_count(&self) -> usize {
		self.tower.apartment_count()
	}

	pub fn room_count(&self) -> usize {
		self.tower.room_count()
	}

	pub fn shaft_slots(&self) -> &[SingleHighriseShaftSlot] {
		&self.tower.shaft_slots
	}

	pub fn stairwells(&self) -> &[ConnectingStairwell] {
		&self.tower.stairwells
	}

	pub fn bridge_passage_id(storey: usize, direction: CardinalFace) -> OpeningId {
		ApartmentMonotower::bridge_passage_id(storey, direction)
	}

	pub fn bridge_passage_opening(
		&self,
		storey: usize,
		direction: CardinalFace,
	) -> Option<&Opening> {
		self.tower.bridge_passage_opening(storey, direction)
	}

	pub fn mapped_bridge_passage(
		&self,
		storey: usize,
		direction: CardinalFace,
	) -> Option<&MappedOpening> {
		self.tower.mapped_bridge_passage(storey, direction)
	}

	pub fn with_wall_material(mut self, material: MaterialRef) -> Self {
		self.wall_material = Some(material);
		self
	}

	/// Fit a requested plan family. This is useful for representative tests and
	/// authored landmarks without adding another development kind.
	pub fn fit_with_plan(
		confines: &Confines,
		noise: NoiseParams,
		plan: SingleHighrisePlan,
	) -> Result<(Self, FillableRegions), FitError> {
		let height = confines.bounds.max.y - confines.bounds.min.y;
		let storey_count = (height / STOREY_HEIGHT).floor() as usize;
		if storey_count < MIN_STOREYS {
			return Err(FitError::TooSmall { reason: "single_highrise_height" });
		}
		let tower = match plan {
			SingleHighrisePlan::Circular => build_circular(confines, noise, storey_count)?,
			SingleHighrisePlan::Rectangular => build_rectangular(confines, noise, storey_count)?,
			SingleHighrisePlan::IFrame => build_i_frame(confines, noise, storey_count)?,
		};
		Ok((
			Self { bounds: confines.bounds, tower, wall_material: None },
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

impl Fit for SingleHighrise {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = SingleHighrisePlan::select(noise.seed, aabb_xz_extent(&confines.bounds));
		Self::fit_with_plan(confines, noise, plan)
	}
}

impl BuildingComponents for SingleHighrise {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.tower.panel_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = self.tower.partition_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.tower.floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		self.tower.roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.tower.stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		self.tower.door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.tower.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.tower.furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		self.tower.furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.tower.label_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let y0 = self.bounds.min.y;
		let height = (self.bounds.max.y - self.bounds.min.y).max(1.0);
		let wall = self.wall_material.clone();
		if let SingleHighriseFloorPlan::IFrame { .. } = &self.tower.floor_plan {
			if let Some(SingleHighriseStorey::IFrame(storey)) = self.tower.storeys.first() {
				let volumes: Vec<_> = storey
					.floor_plan
					.primary_rects
					.iter()
					.map(|rect| {
						MassingVolume::cuboid(rect.to_aabb2(), y0, height)
							.with_material_opt(wall.clone())
					})
					.collect();
				if !volumes.is_empty() {
					return Some(BuildingStructuralLodProbe::from_volumes(volumes));
				}
			}
		}
		let volume = match &self.tower.floor_plan {
			SingleHighriseFloorPlan::Circular { center_xz, radius, .. } => {
				MassingVolume::cylinder(*center_xz, *radius, y0, height)
			}
			SingleHighriseFloorPlan::Rectangular { center_xz, footprint }
			| SingleHighriseFloorPlan::IFrame { center_xz, footprint, .. } => {
				let half = *footprint * 0.5;
				MassingVolume::cuboid(
					Aabb2d { min: *center_xz - half, max: *center_xz + half },
					y0,
					height,
				)
			}
		};
		Some(BuildingStructuralLodProbe::from_volumes([volume.with_material_opt(wall)]))
	}
}

fn build_circular(
	confines: &Confines,
	noise: NoiseParams,
	storey_count: usize,
) -> Result<ApartmentMonotower, FitError> {
	let footprint = aabb_xz_extent(&confines.bounds);
	let radius = footprint.x.min(footprint.y) * CIRCULAR_INSET;
	if radius < 8.0 {
		return Err(FitError::TooSmall { reason: "single_highrise_circular_footprint" });
	}
	let center = confines.center();
	let center_xz = Vec2::new(center.x, center.z);
	let interior_half = radius * CIRCULAR_INSCRIBED_HALF;
	let shaft_half = shaft_half_extent(footprint);
	let shaft_slots =
		vec![SingleHighriseShaftSlot { bounds: centered_bounds(center_xz, shaft_half) }];
	let floor_plan = SingleHighriseFloorPlan::Circular { center_xz, radius, interior_half };
	let mut storeys = Vec::with_capacity(storey_count);
	for storey in 0..storey_count {
		let y0 = confines.bounds.min.y + storey as f32 * STOREY_HEIGHT;
		let openings =
			circular_storey_openings(storey, y0, center_xz, radius, shaft_slots[0].bounds);
		let mut shell_openings = openings.clone();
		mark_shafts_as_slab_cuts(&mut shell_openings);
		let shell = ArcFloor::new(ArcFloorParams {
			center_xz: Vec3::new(center_xz.x, y0, center_xz.y),
			radius,
			storey_height: STOREY_HEIGHT,
			openings: shell_openings,
			floor: ArcFloorSlab::Solid,
			ceiling: if storey + 1 == storey_count {
				ArcFloorSlab::Solid
			} else {
				ArcFloorSlab::None
			},
			style: PartitionStyle::RoughStonework,
		});
		let blocks = circular_apartment_blocks(
			center_xz,
			interior_half,
			shaft_half,
			y0,
			noise_for_cell(noise, storey as i32),
		)?;
		storeys.push(SingleHighriseStorey::Circular {
			shell: Box::new(shell),
			blocks,
			shaft_bounds: shaft_bounds_for_slots(&shaft_slots, y0),
		});
	}
	let stairwells = build_stairwells(
		confines.bounds.min.y,
		storey_count,
		&shaft_slots,
		StairwellKind::Circular,
	);
	Ok(ApartmentMonotower {
		floor_plan,
		storey_height: STOREY_HEIGHT,
		shaft_slots,
		storeys,
		stairwells,
	})
}

fn build_rectangular(
	confines: &Confines,
	noise: NoiseParams,
	storey_count: usize,
) -> Result<ApartmentMonotower, FitError> {
	let available = aabb_xz_extent(&confines.bounds);
	let footprint = available * PLAN_INSET;
	if footprint.x.min(footprint.y) < 12.0 {
		return Err(FitError::TooSmall { reason: "single_highrise_rectangular_footprint" });
	}
	let center = confines.center();
	let center_xz = Vec2::new(center.x, center.z);
	let host = Aabb2d { min: center_xz - footprint * 0.5, max: center_xz + footprint * 0.5 };
	let shaft_half = shaft_half_extent(footprint);
	let shaft_slots =
		vec![SingleHighriseShaftSlot { bounds: centered_bounds(center_xz, shaft_half) }];
	let floor_plan = SingleHighriseFloorPlan::Rectangular { center_xz, footprint };
	let hall_width = (shaft_half * 0.9).clamp(HIGHRISE_MIN_HALL_WIDTH, 3.0);
	let mut storeys = Vec::with_capacity(storey_count);
	for storey in 0..storey_count {
		let y0 = confines.bounds.min.y + storey as f32 * STOREY_HEIGHT;
		let openings = rectangular_storey_openings(storey, y0, host, shaft_slots[0].bounds);
		let mut shell_openings = openings.clone();
		mark_shafts_as_slab_cuts(&mut shell_openings);
		let shell = RectFloor::new(RectFloorParams {
			center_xz: Vec3::new(center_xz.x, y0, center_xz.y),
			footprint,
			storey_height: STOREY_HEIGHT,
			openings: shell_openings,
			floor: RectFloorSlab::Solid,
			ceiling: if storey + 1 == storey_count {
				RectFloorSlab::Solid
			} else {
				RectFloorSlab::None
			},
			style: PanelStyle::RoughStonework,
			..RectFloorParams::default()
		});
		let block_confines = confines_from_plan(host, y0, openings);
		let (apartments, _) = LivableApartments::from_confines_with(
			&block_confines,
			noise_for_cell(noise, storey as i32),
			LivableApartmentsOptions { hall_width: Some(hall_width), targets: None },
		)?;
		storeys.push(SingleHighriseStorey::Rectangular {
			shell: Box::new(shell),
			apartments: Box::new(apartments),
			shaft_bounds: shaft_bounds_for_slots(&shaft_slots, y0),
		});
	}
	let stairwells = build_stairwells(
		confines.bounds.min.y,
		storey_count,
		&shaft_slots,
		StairwellKind::Rectangular,
	);
	Ok(ApartmentMonotower {
		floor_plan,
		storey_height: STOREY_HEIGHT,
		shaft_slots,
		storeys,
		stairwells,
	})
}

fn build_i_frame(
	confines: &Confines,
	noise: NoiseParams,
	storey_count: usize,
) -> Result<ApartmentMonotower, FitError> {
	let available = aabb_xz_extent(&confines.bounds);
	let footprint = available * PLAN_INSET;
	if footprint.x.min(footprint.y) < 12.0 {
		return Err(FitError::TooSmall { reason: "single_highrise_i_frame_footprint" });
	}
	let center = confines.center();
	let center_xz = Vec2::new(center.x, center.z);
	let base_y = confines.bounds.min.y;
	let base_bounds = Aabb3d::from_min_max(
		Vec3::new(center_xz.x - footprint.x * 0.5, base_y, center_xz.y - footprint.y * 0.5),
		Vec3::new(
			center_xz.x + footprint.x * 0.5,
			base_y + STOREY_HEIGHT,
			center_xz.y + footprint.y * 0.5,
		),
	);
	let empty = Confines::new(base_bounds, confines.roll, Openings::new());
	let parameterized = IApartmentParameterized::sample(&empty, noise)?
		.with_minimums(HIGHRISE_MIN_HALL_WIDTH, MIN_SHAFT_SIDE);
	let shaft_requests =
		IApartmentFloorPlan::shaft_requests_for_primary_rects(&parameterized, &empty);
	let shaft_probe_confines = Confines::new(base_bounds, confines.roll, shaft_requests);
	let (shaft_probe, _) =
		IApartmentFloorPlan::from_parameterized(parameterized.clone(), &shaft_probe_confines)?;
	if shaft_probe.shaft_bounds.is_empty() {
		return Err(FitError::InvalidConfines { reason: "single_highrise_i_frame_shafts" });
	}
	let shaft_slots = shaft_probe
		.shaft_bounds
		.iter()
		.map(|bounds| SingleHighriseShaftSlot {
			bounds: Aabb2d {
				min: Vec2::new(bounds.min.x, bounds.min.z),
				max: Vec2::new(bounds.max.x, bounds.max.z),
			},
		})
		.collect::<Vec<_>>();
	let passage_template = i_frame_passages(&shaft_probe.primary_rects, 0, base_y);
	let mut storeys = Vec::with_capacity(storey_count);
	for storey in 0..storey_count {
		let y0 = base_y + storey as f32 * STOREY_HEIGHT;
		let bounds = translate_bounds_y(base_bounds, y0 - base_y);
		let mut openings = shaft_openings_for_slots(&shaft_slots, y0);
		openings.extend(&rebase_bridge_openings(&passage_template, storey, y0 - base_y));
		let floor_confines = Confines::new(bounds, confines.roll, openings);
		let (floor_plan, _) = IApartmentFloorPlan::from_parameterized_with_ceiling(
			parameterized.clone(),
			&floor_confines,
			if storey + 1 == storey_count { IFloorSlab::Solid } else { IFloorSlab::None },
		)?;
		let (full_storey, _) = IApartmentFullStorey::from_floor_plan(
			floor_plan,
			noise_for_cell(noise, storey as i32),
		)?;
		storeys.push(SingleHighriseStorey::IFrame(Box::new(full_storey)));
	}
	let stairwells =
		build_stairwells(base_y, storey_count, &shaft_slots, StairwellKind::Rectangular);
	Ok(ApartmentMonotower {
		floor_plan: SingleHighriseFloorPlan::IFrame { center_xz, footprint, parameterized },
		storey_height: STOREY_HEIGHT,
		shaft_slots,
		storeys,
		stairwells,
	})
}

fn circular_apartment_blocks(
	center: Vec2,
	interior_half: f32,
	shaft_half: f32,
	y0: f32,
	noise: NoiseParams,
) -> Result<Vec<LivableApartments>, FitError> {
	let min = center - Vec2::splat(interior_half);
	let max = center + Vec2::splat(interior_half);
	let shaft = centered_bounds(center, shaft_half);
	// Same four rectangles as the Wizard floor fill: two full-width bars
	// north/south and two side bars spanning the shaft's Z band.
	let regions = [
		Aabb2d { min, max: Vec2::new(max.x, shaft.min.y) },
		Aabb2d { min: Vec2::new(min.x, shaft.max.y), max },
		Aabb2d { min: Vec2::new(min.x, shaft.min.y), max: Vec2::new(shaft.min.x, shaft.max.y) },
		Aabb2d { min: Vec2::new(shaft.max.x, shaft.min.y), max: Vec2::new(max.x, shaft.max.y) },
	];
	let mut blocks = Vec::with_capacity(regions.len());
	for (index, region) in regions.into_iter().enumerate() {
		let access = shaft_access_opening(region, shaft, y0, index);
		let confines = confines_from_plan(region, y0, Openings::new().with(access.0, access.1));
		match LivableApartments::from_confines_with(
			&confines,
			noise_for_cell(noise, index as i32),
			LivableApartmentsOptions { hall_width: Some(HIGHRISE_MIN_HALL_WIDTH), targets: None },
		) {
			Ok((block, _)) => blocks.push(block),
			Err(FitError::TooSmall { .. }) => {}
			Err(err) => return Err(err),
		}
	}
	if blocks.is_empty() {
		return Err(FitError::TooSmall { reason: "single_highrise_circular_interior" });
	}
	Ok(blocks)
}

fn circular_storey_openings(
	storey: usize,
	y0: f32,
	center: Vec2,
	radius: f32,
	shaft: Aabb2d,
) -> Openings {
	let mut openings = shaft_openings_for_slots(&[SingleHighriseShaftSlot { bounds: shaft }], y0);
	for (direction, t) in [
		(CardinalFace::East, 0.0),
		(CardinalFace::North, 0.25),
		(CardinalFace::West, 0.5),
		(CardinalFace::South, 0.75),
	] {
		let id = ApartmentMonotower::bridge_passage_id(storey, direction);
		let (_, opening) = ArcFloor::plan_opening_at_t(
			id.clone(),
			OpeningLabel::Passage,
			Vec3::new(center.x, y0, center.y),
			radius,
			STOREY_HEIGHT,
			t,
		);
		openings.insert(id, opening);
	}
	openings
}

fn rectangular_storey_openings(storey: usize, y0: f32, host: Aabb2d, shaft: Aabb2d) -> Openings {
	let mut openings = shaft_openings_for_slots(&[SingleHighriseShaftSlot { bounds: shaft }], y0);
	for direction in cardinal_directions() {
		let id = ApartmentMonotower::bridge_passage_id(storey, direction);
		openings.insert(id, cardinal_passage(host, y0, direction));
	}
	openings
}

fn i_frame_passages(rects: &[crate::shells::IFloorPlanRect], storey: usize, y0: f32) -> Openings {
	let mut openings = Openings::new();
	for direction in cardinal_directions() {
		let Some(rect) = outermost_rect(rects, direction) else {
			continue;
		};
		let host = rect.to_aabb2();
		let id = ApartmentMonotower::bridge_passage_id(storey, direction);
		openings.insert(id, cardinal_passage(host, y0, direction));
	}
	openings
}

fn outermost_rect(
	rects: &[crate::shells::IFloorPlanRect],
	direction: CardinalFace,
) -> Option<&crate::shells::IFloorPlanRect> {
	rects.iter().max_by(|a, b| {
		let av = match direction {
			CardinalFace::West => -a.min_x,
			CardinalFace::East => a.max_x,
			CardinalFace::South => -a.min_z,
			CardinalFace::North => a.max_z,
		};
		let bv = match direction {
			CardinalFace::West => -b.min_x,
			CardinalFace::East => b.max_x,
			CardinalFace::South => -b.min_z,
			CardinalFace::North => b.max_z,
		};
		av.total_cmp(&bv)
	})
}

fn rebase_bridge_openings(template: &Openings, storey: usize, dy: f32) -> Openings {
	let mut openings = Openings::new();
	for direction in cardinal_directions() {
		let base_id = ApartmentMonotower::bridge_passage_id(0, direction);
		let Some(opening) = template.get(&base_id) else {
			continue;
		};
		let min = Vec3::from(opening.bounds.min) + Vec3::Y * dy;
		let max = Vec3::from(opening.bounds.max) + Vec3::Y * dy;
		openings.insert(
			ApartmentMonotower::bridge_passage_id(storey, direction),
			Opening::new(Aabb3d::from_min_max(min, max), OpeningLabel::Passage),
		);
	}
	openings
}

fn shaft_openings_for_slots(slots: &[SingleHighriseShaftSlot], y0: f32) -> Openings {
	let mut openings = Openings::new();
	for (index, slot) in slots.iter().enumerate() {
		openings.insert(
			OpeningId::scoped(SCOPE, "shaft", index.to_string()),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(slot.bounds.min.x, y0, slot.bounds.min.y),
					Vec3::new(slot.bounds.max.x, y0 + STOREY_HEIGHT, slot.bounds.max.y),
				),
				OpeningLabel::Shaft,
			),
		);
	}
	openings
}

fn shaft_bounds_for_slots(slots: &[SingleHighriseShaftSlot], y0: f32) -> Vec<Aabb3d> {
	slots
		.iter()
		.map(|slot| {
			Aabb3d::from_min_max(
				Vec3::new(slot.bounds.min.x, y0, slot.bounds.min.y),
				Vec3::new(slot.bounds.max.x, y0 + STOREY_HEIGHT, slot.bounds.max.y),
			)
		})
		.collect()
}

fn mark_shafts_as_slab_cuts(openings: &mut Openings) {
	for opening in openings.openings.values_mut() {
		if matches!(opening.label, OpeningLabel::Shaft) {
			opening.label = OpeningLabel::Custom("shaft_slab_cut".to_owned());
		}
	}
}

fn build_stairwells(
	base_y: f32,
	storey_count: usize,
	slots: &[SingleHighriseShaftSlot],
	kind: StairwellKind,
) -> Vec<ConnectingStairwell> {
	let mut stairwells = Vec::with_capacity(storey_count.saturating_sub(1) * slots.len());
	for storey in 0..storey_count.saturating_sub(1) {
		for slot in slots {
			let y0 = base_y + storey as f32 * STOREY_HEIGHT;
			// Stacked wells share one walk face, so each arrival is the next
			// well's run-in; only the top well needs its own landing. The hall
			// ring reaches every shaft face, so the closed faces can be lined.
			let well = WellAabb::from_plan(
				Vec3::new(slot.bounds.min.x, y0, slot.bounds.min.y),
				Vec3::new(slot.bounds.max.x, y0 + STOREY_HEIGHT, slot.bounds.max.y),
				WellSide::PosX,
				WellSide::PosX,
				STAIR_TREAD_FILL,
			);
			stairwells.push(
				ConnectingStairwell::from_well_kind(PanelStyle::RoughStonework, well, kind)
					.with_upper_landing(storey + 2 == storey_count)
					.with_shaft_walls(true),
			);
		}
	}
	stairwells
}

fn shaft_access_opening(
	region: Aabb2d,
	shaft: Aabb2d,
	y0: f32,
	index: usize,
) -> (OpeningId, Opening) {
	let overlap_x_min = region.min.x.max(shaft.min.x);
	let overlap_x_max = region.max.x.min(shaft.max.x);
	let overlap_z_min = region.min.y.max(shaft.min.y);
	let overlap_z_max = region.max.y.min(shaft.max.y);
	let pad = 0.2;
	let bounds = if overlap_x_max - overlap_x_min > 0.5 {
		let x = (overlap_x_min + overlap_x_max) * 0.5;
		let z = if region.max.y <= shaft.min.y + 1e-3 { shaft.min.y } else { shaft.max.y };
		Aabb3d::from_min_max(
			Vec3::new(x - BRIDGE_PASSAGE_WIDTH * 0.5, y0, z - pad),
			Vec3::new(x + BRIDGE_PASSAGE_WIDTH * 0.5, y0 + 2.2, z + pad),
		)
	} else {
		let z = (overlap_z_min + overlap_z_max) * 0.5;
		let x = if region.max.x <= shaft.min.x + 1e-3 { shaft.min.x } else { shaft.max.x };
		Aabb3d::from_min_max(
			Vec3::new(x - pad, y0, z - BRIDGE_PASSAGE_WIDTH * 0.5),
			Vec3::new(x + pad, y0 + 2.2, z + BRIDGE_PASSAGE_WIDTH * 0.5),
		)
	};
	(
		OpeningId::scoped(SCOPE, "shaft_access", index.to_string()),
		Opening::new(bounds, OpeningLabel::Shaft),
	)
}

fn cardinal_passage(host: Aabb2d, y0: f32, direction: CardinalFace) -> Opening {
	let center = host.center();
	let half_w = BRIDGE_PASSAGE_WIDTH * 0.5;
	let half_d = BRIDGE_PASSAGE_DEPTH * 0.5;
	let y1 = y0 + STOREY_HEIGHT.min(2.4);
	let bounds = match direction {
		CardinalFace::West => Aabb3d::from_min_max(
			Vec3::new(host.min.x - half_d, y0, center.y - half_w),
			Vec3::new(host.min.x + half_d, y1, center.y + half_w),
		),
		CardinalFace::East => Aabb3d::from_min_max(
			Vec3::new(host.max.x - half_d, y0, center.y - half_w),
			Vec3::new(host.max.x + half_d, y1, center.y + half_w),
		),
		CardinalFace::South => Aabb3d::from_min_max(
			Vec3::new(center.x - half_w, y0, host.min.y - half_d),
			Vec3::new(center.x + half_w, y1, host.min.y + half_d),
		),
		CardinalFace::North => Aabb3d::from_min_max(
			Vec3::new(center.x - half_w, y0, host.max.y - half_d),
			Vec3::new(center.x + half_w, y1, host.max.y + half_d),
		),
	};
	Opening::new(bounds, OpeningLabel::Passage)
}

fn confines_from_plan(host: Aabb2d, y0: f32, openings: Openings) -> Confines {
	Confines::new(
		Aabb3d::from_min_max(
			Vec3::new(host.min.x, y0, host.min.y),
			Vec3::new(host.max.x, y0 + STOREY_HEIGHT, host.max.y),
		),
		0.0,
		openings,
	)
}

fn centered_bounds(center: Vec2, half: f32) -> Aabb2d {
	Aabb2d { min: center - Vec2::splat(half), max: center + Vec2::splat(half) }
}

fn shaft_half_extent(footprint: Vec2) -> f32 {
	(footprint.x.min(footprint.y) * SHAFT_FRACTION)
		.clamp(MIN_SHAFT_SIDE * 0.5, MAX_SHAFT_SIDE * 0.5)
}

fn translate_bounds_y(bounds: Aabb3d, dy: f32) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::from(bounds.min) + Vec3::Y * dy,
		Vec3::from(bounds.max) + Vec3::Y * dy,
	)
}

fn cardinal_directions() -> [CardinalFace; 4] {
	[CardinalFace::West, CardinalFace::East, CardinalFace::South, CardinalFace::North]
}

fn direction_name(direction: CardinalFace) -> &'static str {
	match direction {
		CardinalFace::West => "west",
		CardinalFace::East => "east",
		CardinalFace::South => "south",
		CardinalFace::North => "north",
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn confines(footprint: Vec2) -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-footprint.x * 0.5, 0.0, -footprint.y * 0.5),
			Vec3::new(footprint.x * 0.5, 48.0, footprint.y * 0.5),
		))
	}

	#[test]
	fn all_variants_fit_with_real_apartments_and_rooms() -> anyhow::Result<()> {
		for plan in [
			SingleHighrisePlan::Circular,
			SingleHighrisePlan::Rectangular,
			SingleHighrisePlan::IFrame,
		] {
			let (highrise, _) = SingleHighrise::fit_with_plan(
				&confines(Vec2::new(44.0, 38.0)),
				NoiseParams { seed: 17, ..NoiseParams::default() },
				plan,
			)?;
			assert_eq!(highrise.plan(), plan);
			assert_eq!(highrise.storey_count(), 15);
			assert!(highrise.apartment_count() > 0, "{plan:?} has no apartments");
			assert!(highrise.room_count() > 0, "{plan:?} has no rooms");
		}
		Ok(())
	}

	#[test]
	fn selection_is_deterministic_and_uses_aspect() {
		let square = Vec2::splat(40.0);
		assert_eq!(SingleHighrisePlan::select(0, square), SingleHighrisePlan::select(0, square));
		assert_eq!(SingleHighrisePlan::select(0, square), SingleHighrisePlan::Circular);
		assert_eq!(SingleHighrisePlan::select(1, square), SingleHighrisePlan::Rectangular);
		assert_eq!(SingleHighrisePlan::select(2, square), SingleHighrisePlan::IFrame);
		assert_ne!(
			SingleHighrisePlan::select(0, square),
			SingleHighrisePlan::select(0, Vec2::new(64.0, 32.0))
		);
	}

	#[test]
	fn shafts_and_stairs_replay_aligned_slots() -> anyhow::Result<()> {
		for plan in [
			SingleHighrisePlan::Circular,
			SingleHighrisePlan::Rectangular,
			SingleHighrisePlan::IFrame,
		] {
			let (highrise, _) = SingleHighrise::fit_with_plan(
				&confines(Vec2::splat(44.0)),
				NoiseParams::default(),
				plan,
			)?;
			let slots = highrise.shaft_slots();
			assert!(!slots.is_empty());
			assert_eq!(highrise.stairwells().len(), (highrise.storey_count() - 1) * slots.len());
			for storey in &highrise.tower.storeys {
				let shafts = storey.shaft_bounds();
				assert_eq!(shafts.len(), slots.len());
				for (shaft, slot) in shafts.iter().zip(slots) {
					assert!((shaft.min.x - slot.bounds.min.x).abs() < 1e-3);
					assert!((shaft.max.z - slot.bounds.max.y).abs() < 1e-3);
				}
			}
			for (index, stair) in highrise.stairwells().iter().enumerate() {
				let slot = slots[index % slots.len()];
				let well = stair.well();
				assert!((well.min().x - slot.bounds.min.x).abs() < 1e-3);
				assert!((well.max().z - slot.bounds.max.y).abs() < 1e-3);
			}
		}
		Ok(())
	}

	#[test]
	fn shafts_and_halls_meet_the_highrise_floors() -> anyhow::Result<()> {
		for plan in [
			SingleHighrisePlan::Circular,
			SingleHighrisePlan::Rectangular,
			SingleHighrisePlan::IFrame,
		] {
			let (highrise, _) = SingleHighrise::fit_with_plan(
				&confines(Vec2::splat(44.0)),
				NoiseParams::default(),
				plan,
			)?;
			for slot in highrise.shaft_slots() {
				let side = (slot.bounds.max - slot.bounds.min).min_element();
				anyhow::ensure!(side >= MIN_SHAFT_SIDE - 1e-3, "{plan:?} shaft is {side:.2} m");
			}
			let floor_plan = &highrise.tower.floor_plan;
			if let SingleHighriseFloorPlan::IFrame { parameterized, .. } = floor_plan {
				anyhow::ensure!(parameterized.hall_width >= HIGHRISE_MIN_HALL_WIDTH - 1e-3);
			}
		}
		Ok(())
	}

	#[test]
	fn every_stair_arrival_lands_on_the_next_run_in() -> anyhow::Result<()> {
		for plan in [
			SingleHighrisePlan::Circular,
			SingleHighrisePlan::Rectangular,
			SingleHighrisePlan::IFrame,
		] {
			let (highrise, _) = SingleHighrise::fit_with_plan(
				&confines(Vec2::splat(44.0)),
				NoiseParams::default(),
				plan,
			)?;
			let slots = highrise.shaft_slots().len();
			let wells = highrise.stairwells();
			for (index, stair) in wells.iter().enumerate() {
				let well = stair.well();
				anyhow::ensure!(well.walk_on == well.walk_off, "{plan:?} well {index} crosses");
				anyhow::ensure!(!stair.shaft_walls().is_empty(), "{plan:?} well {index} is open");
				let Some(next) = wells.get(index + slots) else {
					anyhow::ensure!(stair.upper_landing().is_some(), "{plan:?} top has no landing");
					continue;
				};
				anyhow::ensure!(
					next.well().walk_on == well.walk_off
						&& (next.well().bottom_y() - well.top_y()).abs() < 1e-3,
					"{plan:?} well {index} arrives where no run-in waits"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn every_storey_exposes_mapped_cardinal_bridge_passages() -> anyhow::Result<()> {
		for plan in [
			SingleHighrisePlan::Circular,
			SingleHighrisePlan::Rectangular,
			SingleHighrisePlan::IFrame,
		] {
			let (highrise, _) = SingleHighrise::fit_with_plan(
				&confines(Vec2::splat(44.0)),
				NoiseParams::default(),
				plan,
			)?;
			for storey in 0..highrise.storey_count() {
				for direction in cardinal_directions() {
					assert!(
						highrise.bridge_passage_opening(storey, direction).is_some(),
						"{plan:?} storey {storey} missing {direction:?}"
					);
					assert!(
						highrise.mapped_bridge_passage(storey, direction).is_some(),
						"{plan:?} storey {storey} did not map {direction:?}"
					);
				}
			}
		}
		Ok(())
	}
}
