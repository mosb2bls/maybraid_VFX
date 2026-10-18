//! Ring fort: a curtain-wall courtyard ring with circular or trazaloid corner keeps.
//!
//! The ring is 2–4 storeys. A full-ring terrace deck is the ceiling: stairwells
//! climb onto it, corner keeps sit on the gallery corners, and two colonnade
//! lines between the keeps carry a hipped pitched roof. Topology is a star: the
//! ring is the hub, and each keep joins it at a gallery corner.

use std::sync::Arc;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::furniture::FurnitureNode;
use building_components::panels::PanelStyle;
use building_components::{
	BuildingComponents, BuildingStructuralLodProbe, JointNode, Layers, PanelNode,
};
use buildings::usage_areas::furniture_util::{
	chest_in_confines_clear, pillar_keep_outs, PILLAR_CHEST_PAD,
};
use buildings::{
	Confines, ConnectingStairwell, EndCap, FillableRegions, Fit, FitError, FittedRectangle,
	LesHallesFloorPlan, Opening, OpeningId, OpeningLabel, Openings, Overhang, PanelPillar,
	PanelPillarLine, PanelPoint, RectRingFloor, RectRingFloorParams, RectRingFloorSlab,
	RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams, StairwellKind, WellAabb,
};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::{NoiseConfig, NoiseParams};

use super::curtain_ring::CurtainRing;
use super::keep::{CircularTower, RingFortKeep, TrazaloidTower};
use crate::connected::{ConnectedDevelopment, DevelopmentEdge};
use crate::development::les_halles::{courtyard_well_side, MixedUseLesHallesHost};
use crate::placed::BuildingFootprint;

/// Historical name for [`RingFortKeep`].
pub type RingFortTower = RingFortKeep;

/// Minimum curtain-wall plan so a deep gallery + courtyard still fit.
const MIN_RING_PLAN: f32 = 80.0;
const TOWER_STOREY_MIN: usize = 5;
const TOWER_STOREY_MAX: usize = 10;
const CIRCULAR_RADIUS_MIN: f32 = 11.0;
const CIRCULAR_RADIUS_MAX: f32 = 16.0;
const TRAZALOID_FOOT_MIN: f32 = 14.0;
const TRAZALOID_FOOT_MAX: f32 = 22.0;

const SALT_KIND: f32 = 53.0;
const SALT_FLOORS: f32 = 59.0;
const SALT_SIZE: f32 = 61.0;

const PILLAR_INSET: f32 = 1.35;
/// Clearance from a keep wall to the first colonnade pier (and hip end).
const TOWER_CLEAR: f32 = 10.0;
/// Pull both paired rows and their hip inside the courtyard's along-axis span.
const COLONNADE_INNER_END_INSET: f32 = 4.0;
const PILLAR_WIDTH: f32 = 0.7;
const PILLAR_SPACING: f32 = 4.5;
const PILLAR_HEIGHT: f32 = 3.0;
const MIN_COLONNADE_RUN: f32 = 8.0;
const ROOF_LINE_OVERHANG: f32 = 0.35;
const TERRACE_TREAD_FILL: f32 = 0.85;
const TERRACE_PARAPET: f32 = 0.55;
const TERRACE_SLAB: f32 = 0.32;
const TERRACE_CHEST_HEIGHT: f32 = 2.4;
const COLONNADE_CHEST_CHUNK: f32 = 12.0;
const KEEP_APRON_DEPTH: f32 = 3.6;
const KEEP_APRON_ALONG: f32 = 4.5;

const CORNERS: [(f32, f32); 4] = [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)];

/// Corner join from a keep onto the courtyard ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RingFortJoin;

/// One site in a [`RingFortBuilding`]: the courtyard curtain or a corner keep.
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortSite {
	Ring(Box<CurtainRing>),
	Keep(RingFortKeep),
}

/// Sibling LOD host emitted by [`RingFortBuilding`].
#[derive(Debug, Clone, PartialEq)]
pub enum RingFortHost {
	Ring(Box<MixedUseLesHallesHost>),
	Circular(Arc<CircularTower>),
	Trazaloid(Arc<TrazaloidTower>),
	KeepStairwell(buildings::ConnectingStairwell),
	Terrace(GalleryTerrace),
	TerraceStairwell(buildings::ConnectingStairwell),
	GalleryColonnade(GalleryColonnade),
	GalleryRoof(RectangularPitchedRoofComplex),
}

/// Walkable ceiling deck on top of the curtain ring.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryTerrace {
	deck: RectRingFloor,
	wall_material: Option<MaterialRef>,
	residual_chests: Vec<FurnitureNode>,
}

impl GalleryTerrace {
	pub fn deck(&self) -> &RectRingFloor {
		&self.deck
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}

	fn with_chests(mut self, residual_chests: Vec<FurnitureNode>) -> Self {
		self.residual_chests = residual_chests;
		self
	}
}

impl BuildingComponents for GalleryTerrace {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.deck.panel_nodes_for_level(level);
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.deck.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		if !matches!(level, LodSceneLevel::High) {
			return Layers::new();
		}
		Layers::from_free(self.residual_chests.clone())
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		let mut probe = self.deck.structural_lod()?;
		if let Some(material) = &self.wall_material {
			for volume in &mut probe.volumes {
				volume.material = Some(material.clone());
			}
		}
		Some(probe)
	}
}

/// Two colonnade lines on the ring terrace, between the corner keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryColonnade {
	pillars: PanelPillarLine,
	soffits: Vec<FittedRectangle>,
	wall_material: Option<MaterialRef>,
}

impl GalleryColonnade {
	pub fn pillars(&self) -> &PanelPillarLine {
		&self.pillars
	}

	pub fn with_wall_material(mut self, wall: MaterialRef) -> Self {
		self.wall_material = Some(wall);
		self
	}
}

impl BuildingComponents for GalleryColonnade {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.pillars.panel_nodes_for_level(level);
		for soffit in &self.soffits {
			out.extend(soffit.panel_nodes_for_level(level));
		}
		if let Some(material) = &self.wall_material {
			out = out.with_material(material.clone());
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.pillars.joint_nodes_for_level(level);
		for soffit in &self.soffits {
			out.extend(soffit.joint_nodes_for_level(level));
		}
		out
	}
}

/// Courtyard ring with four corner keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct RingFortBuilding {
	pub connected: ConnectedDevelopment<RingFortSite, RingFortJoin>,
	pub terrace: GalleryTerrace,
	pub terrace_stairs: Vec<ConnectingStairwell>,
	pub colonnade: GalleryColonnade,
	pub roof: RectangularPitchedRoofComplex,
}

impl RingFortBuilding {
	pub fn ring(&self) -> Option<&CurtainRing> {
		self.connected.nodes.iter().find_map(|site| match site {
			RingFortSite::Ring(ring) => Some(ring.as_ref()),
			RingFortSite::Keep(_) => None,
		})
	}

	pub fn keeps(&self) -> impl Iterator<Item = &RingFortKeep> {
		self.connected.nodes.iter().filter_map(|site| match site {
			RingFortSite::Keep(keep) => Some(keep),
			RingFortSite::Ring(_) => None,
		})
	}

	pub fn towers(&self) -> impl Iterator<Item = &RingFortKeep> {
		self.keeps()
	}

	pub fn hosts(&self) -> Vec<RingFortHost> {
		let mut out = Vec::new();
		for site in &self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					out.extend(
						ring.hosts_without_roof()
							.into_iter()
							.map(|host| RingFortHost::Ring(Box::new(host))),
					);
				}
				RingFortSite::Keep(RingFortKeep::Circular(keep)) => {
					out.push(RingFortHost::Circular(Arc::new(keep.shell.clone())));
					out.extend(keep.stairwells.iter().cloned().map(RingFortHost::KeepStairwell));
				}
				RingFortSite::Keep(RingFortKeep::Trazaloid(keep)) => {
					out.push(RingFortHost::Trazaloid(Arc::new(keep.shell.clone())));
					out.extend(keep.stairwells.iter().cloned().map(RingFortHost::KeepStairwell));
				}
			}
		}
		out.push(RingFortHost::Terrace(self.terrace.clone()));
		out.extend(self.terrace_stairs.iter().cloned().map(RingFortHost::TerraceStairwell));
		out.push(RingFortHost::GalleryColonnade(self.colonnade.clone()));
		out.push(RingFortHost::GalleryRoof(self.roof.clone()));
		out
	}

	pub fn with_finish(mut self, wall: MaterialRef, roof: MaterialRef) -> Self {
		for site in &mut self.connected.nodes {
			match site {
				RingFortSite::Ring(ring) => {
					**ring = ring.as_ref().clone().with_finish(wall.clone(), roof.clone());
				}
				RingFortSite::Keep(keep) => {
					*keep = keep.clone().with_wall_material(wall.clone());
				}
			}
		}
		self.terrace = self.terrace.clone().with_wall_material(wall.clone());
		self.terrace_stairs = self
			.terrace_stairs
			.iter()
			.cloned()
			.map(|stair| crate::finish::paint_stairwell(stair, wall.clone()))
			.collect();
		self.colonnade = self.colonnade.clone().with_wall_material(wall);
		self.roof = self.roof.clone().with_surface_material(roof);
		self
	}
}

impl BuildingFootprint for RingFortBuilding {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		let mut rects = Vec::new();
		if let Some(ring) = self.ring() {
			rects.extend(ring.footprint_rects());
		}
		for keep in self.keeps() {
			let c = keep.center_xz();
			let half = keep.plan_half_extent();
			rects.push(Aabb2d::new(Vec2::new(c.x, c.z), Vec2::splat(half)));
		}
		rects
	}
}

impl Fit for RingFortBuilding {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let extent = Vec3::from(confines.bounds.max - confines.bounds.min);
		if extent.x + 1e-3 < MIN_RING_PLAN || extent.z + 1e-3 < MIN_RING_PLAN {
			return Err(FitError::TooSmall { reason: "ring_fort_ring" });
		}

		let (mut ring, _) = CurtainRing::fit(confines, noise)?;
		let last = ring
			.last_plan()
			.ok_or(FitError::TooSmall { reason: "ring_fort_storeys" })?
			.clone();
		let max_half = (ring.gallery_width() * 0.48).max(4.0);
		let cfg = NoiseConfig::new(noise);
		let center = confines.center();

		let mut keeps = Vec::with_capacity(4);
		for (i, &(sx, sz)) in CORNERS.iter().enumerate() {
			let origin = ring
				.keep_anchor(sx, sz)
				.ok_or(FitError::TooSmall { reason: "ring_fort_storeys" })?;
			let spec = sample_keep_spec(&cfg, center, i, max_half);
			keeps.push(spec.build(origin, (sx, sz)));
		}
		let (colonnade, roof) = gallery_colonnade_and_roof(&last, &keeps);
		let shafts = terrace_shafts(&last);
		open_last_storey_for_terrace(&mut ring);
		let terrace = gallery_terrace(&last, &shafts).with_chests(terrace_chests(
			&last,
			&keeps,
			&shafts,
			colonnade.pillars(),
			noise,
		));
		let terrace_stairs = terrace_stairs(&last, &shafts);

		let mut nodes = Vec::with_capacity(5);
		nodes.push(RingFortSite::Ring(Box::new(ring)));
		let mut edges = Vec::with_capacity(4);
		for keep in keeps {
			let keep_i = nodes.len();
			nodes.push(RingFortSite::Keep(keep));
			edges.push(DevelopmentEdge::new(0, keep_i, RingFortJoin));
		}

		Ok((
			Self {
				connected: ConnectedDevelopment::new(confines.bounds, nodes, edges),
				terrace,
				terrace_stairs,
				colonnade,
				roof,
			},
			FillableRegions { within: Vec::new(), atop: Vec::new() },
		))
	}
}

struct KeepSpec {
	circular: bool,
	floors: usize,
	size: f32,
}

impl KeepSpec {
	fn build(&self, origin: Vec3, corner: (f32, f32)) -> RingFortKeep {
		if self.circular {
			RingFortKeep::circular(origin, self.size, self.floors)
		} else {
			RingFortKeep::trazaloid(origin, self.size, self.floors, corner)
		}
	}
}

fn sample_keep_spec(cfg: &NoiseConfig, center: Vec3, index: usize, max_half: f32) -> KeepSpec {
	let salt = index as f32;
	let circular = cfg.sample_unit_4d(center.x, center.y, center.z, SALT_KIND + salt) < 0.5;
	let floors = TOWER_STOREY_MIN
		+ cfg.sample_range_usize_4d(
			0,
			TOWER_STOREY_MAX - TOWER_STOREY_MIN + 1,
			center.x,
			center.y,
			center.z,
			SALT_FLOORS + salt,
		);
	if circular {
		let lo = CIRCULAR_RADIUS_MIN.min(max_half);
		let hi = CIRCULAR_RADIUS_MAX.min(max_half).max(lo);
		let radius =
			cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		KeepSpec { circular: true, floors, size: radius }
	} else {
		let max_foot = (max_half * 2.0).max(TRAZALOID_FOOT_MIN);
		let lo = TRAZALOID_FOOT_MIN.min(max_foot);
		let hi = TRAZALOID_FOOT_MAX.min(max_foot).max(lo);
		let foot = cfg.sample_range_f32_4d(lo, hi, center.x, center.y, center.z, SALT_SIZE + salt);
		KeepSpec { circular: false, floors, size: foot }
	}
}

/// Cloister: two paired pillar lines per gallery segment, hip roof sitting on them.
fn gallery_colonnade_and_roof(
	plan: &LesHallesFloorPlan,
	keeps: &[RingFortKeep],
) -> (GalleryColonnade, RectangularPitchedRoofComplex) {
	let cx = plan.center_xz.x;
	let cz = plan.center_xz.z;
	let y0 = plan.center_xz.y + plan.storey_height;
	let ox = plan.outer.x * 0.5;
	let oz = plan.outer.y * 0.5;
	let ix = plan.courtyard.x * 0.5;
	let iz = plan.courtyard.y * 0.5;
	let gx = (ox - ix).max(0.0);
	let gz = (oz - iz).max(0.0);
	let y1 = y0 + PILLAR_HEIGHT;
	let rise = (gx.min(gz) * 0.45).clamp(2.0, 3.6);
	let y2 = y1 + rise;

	let keep = |sx: f32, sz: f32| {
		let i = CORNERS.iter().position(|&c| c == (sx, sz)).unwrap_or(0);
		&keeps[i]
	};
	let nw = keep(-1.0, 1.0);
	let ne = keep(1.0, 1.0);
	let se = keep(1.0, -1.0);
	let sw = keep(-1.0, -1.0);

	let mut pillars = Vec::new();
	let mut soffits = Vec::new();
	let mut volumes = Vec::new();
	let mut push_segment =
		|along_x: bool, a: &RingFortKeep, b: &RingFortKeep, inner: f32, outer: f32| {
			let along_center = if along_x { cx } else { cz };
			let along_inner_half = if along_x { ix } else { iz };
			let Some((start, end)) = run_between(a, b, along_x, along_center, along_inner_half)
			else {
				return;
			};
			let (inner, outer) = if inner < outer { (inner, outer) } else { (outer, inner) };
			let inner_line = if along_x {
				PanelPillarLine::along_rough_stone(
					Vec3::new(start, y0, inner),
					Vec3::new(end, y0, inner),
					PILLAR_HEIGHT,
					PILLAR_WIDTH,
					PILLAR_SPACING,
				)
			} else {
				PanelPillarLine::along_rough_stone(
					Vec3::new(inner, y0, start),
					Vec3::new(inner, y0, end),
					PILLAR_HEIGHT,
					PILLAR_WIDTH,
					PILLAR_SPACING,
				)
			};
			if inner_line.is_empty() {
				return;
			}
			let outer_line: Vec<PanelPillar> = inner_line
				.pillars
				.iter()
				.map(|p| {
					let mut c = p.center;
					if along_x {
						c.z = outer;
					} else {
						c.x = outer;
					}
					PanelPillar::rough_stone(c, p.width, p.height)
				})
				.collect();
			pillars.extend(inner_line.pillars);
			pillars.extend(outer_line);
			let t = TERRACE_SLAB;
			if along_x {
				soffits.push(FittedRectangle::new(
					PanelStyle::RoughStonework,
					PanelPoint::new(Vec3::new(start, y1, inner), t),
					PanelPoint::new(Vec3::new(end, y1, inner), t),
					PanelPoint::new(Vec3::new(start, y1, outer), t),
					PanelPoint::new(Vec3::new(end, y1, outer), t),
				));
				volumes.push(Aabb3d::from_min_max(
					Vec3::new(start, y1, inner - ROOF_LINE_OVERHANG),
					Vec3::new(end, y2, outer + ROOF_LINE_OVERHANG),
				));
			} else {
				soffits.push(FittedRectangle::new(
					PanelStyle::RoughStonework,
					PanelPoint::new(Vec3::new(inner, y1, start), t),
					PanelPoint::new(Vec3::new(inner, y1, end), t),
					PanelPoint::new(Vec3::new(outer, y1, start), t),
					PanelPoint::new(Vec3::new(outer, y1, end), t),
				));
				volumes.push(Aabb3d::from_min_max(
					Vec3::new(inner - ROOF_LINE_OVERHANG, y1, start),
					Vec3::new(outer + ROOF_LINE_OVERHANG, y2, end),
				));
			}
		};

	push_segment(true, nw, ne, cz + iz + PILLAR_INSET, cz + oz - PILLAR_INSET);
	push_segment(true, sw, se, cz - iz - PILLAR_INSET, cz - oz + PILLAR_INSET);
	push_segment(false, sw, nw, cx - ix - PILLAR_INSET, cx - ox + PILLAR_INSET);
	push_segment(false, se, ne, cx + ix + PILLAR_INSET, cx + ox - PILLAR_INSET);

	let roof = RectangularPitchedRoofComplexParams::new(volumes)
		.overhang(Overhang::Fixed(0.45))
		.end_cap(EndCap::Hip)
		.style(PanelStyle::ShepherdsThatch)
		.build();
	(
		GalleryColonnade { pillars: PanelPillarLine::new(pillars), soffits, wall_material: None },
		roof,
	)
}

fn run_between(
	a: &RingFortKeep,
	b: &RingFortKeep,
	along_x: bool,
	inner_center: f32,
	inner_half: f32,
) -> Option<(f32, f32)> {
	let coord = |k: &RingFortKeep| {
		if along_x {
			k.center_xz().x
		} else {
			k.center_xz().z
		}
	};
	let (lo, hi) = if coord(a) <= coord(b) { (a, b) } else { (b, a) };
	let tower_start = coord(lo) + lo.plan_half_extent() + TOWER_CLEAR;
	let tower_end = coord(hi) - hi.plan_half_extent() - TOWER_CLEAR;
	let inner_start = inner_center - inner_half + COLONNADE_INNER_END_INSET;
	let inner_end = inner_center + inner_half - COLONNADE_INNER_END_INSET;
	let start = tower_start.max(inner_start);
	let end = tower_end.min(inner_end);
	(end - start > MIN_COLONNADE_RUN).then_some((start, end))
}

/// Drop the last gallery ceiling so [`GalleryTerrace`] is the ring’s only eave slab.
///
/// Les Halles only ceilings the gallery band, not the balcony. Rebuilding that
/// shell with extra mid-side shafts fights the façade openings, so the terrace
/// owns the stair holes and the last storey just stops enclosing the deck.
fn open_last_storey_for_terrace(ring: &mut CurtainRing) {
	let Some(last) = ring.halles.tower.floors.last_mut() else {
		return;
	};
	let plan = last.floor_plan_mut();
	let mut params = plan.gallery.params().clone();
	params.ceiling = RectRingFloorSlab::None;
	plan.gallery = params.build();
	plan.ceiling = RectRingFloorSlab::None;
}

/// Last-storey shafts, lifted onto the terrace so the climb continues the wells below.
fn terrace_shafts(plan: &LesHallesFloorPlan) -> Vec<Aabb3d> {
	let y0 = plan.center_xz.y;
	let y1 = y0 + plan.storey_height;
	plan.shaft_bounds
		.iter()
		.map(|shaft| {
			let min = Vec3::from(shaft.min);
			let max = Vec3::from(shaft.max);
			Aabb3d::from_min_max(Vec3::new(min.x, y0, min.z), Vec3::new(max.x, y1, max.z))
		})
		.collect()
}

fn gallery_terrace(plan: &LesHallesFloorPlan, shafts: &[Aabb3d]) -> GalleryTerrace {
	let eave = Vec3::new(plan.center_xz.x, plan.center_xz.y + plan.storey_height, plan.center_xz.z);
	let mut openings = Openings::new();
	for (i, shaft) in shafts.iter().enumerate() {
		openings.insert(
			OpeningId::new(format!("terrace_shaft_{i}")),
			Opening::new(*shaft, OpeningLabel::Shaft),
		);
	}
	GalleryTerrace {
		deck: RectRingFloorParams::new(eave, plan.outer, plan.courtyard, TERRACE_PARAPET)
			.floor(RectRingFloorSlab::Solid)
			.ceiling(RectRingFloorSlab::None)
			.inner_walls(false)
			.openings(openings)
			.style(PanelStyle::RoughStonework)
			.joint_thickness(TERRACE_SLAB)
			.build(),
		wall_material: None,
		residual_chests: Vec::new(),
	}
}

fn terrace_stairs(plan: &LesHallesFloorPlan, shafts: &[Aabb3d]) -> Vec<ConnectingStairwell> {
	let y0 = plan.center_xz.y;
	let y1 = y0 + plan.storey_height;
	shafts
		.iter()
		.map(|shaft| {
			let min = Vec3::from(shaft.min);
			let max = Vec3::from(shaft.max);
			let side = courtyard_well_side(plan.center_xz, *shaft);
			let well = WellAabb::from_plan(
				Vec3::new(min.x, y0, min.z),
				Vec3::new(max.x, y1, max.z),
				side,
				side,
				TERRACE_TREAD_FILL,
			);
			ConnectingStairwell::from_well_kind(
				PanelStyle::RoughStonework,
				well,
				StairwellKind::Rectangular,
			)
			.with_upper_landing(true)
		})
		.collect()
}

fn terrace_chests(
	plan: &LesHallesFloorPlan,
	keeps: &[RingFortKeep],
	shafts: &[Aabb3d],
	colonnade: &PanelPillarLine,
	noise: NoiseParams,
) -> Vec<FurnitureNode> {
	let y0 = plan.center_xz.y + plan.storey_height;
	let y1 = y0 + TERRACE_CHEST_HEIGHT;
	let keep_outs = pillar_keep_outs(colonnade.pillars.iter(), PILLAR_CHEST_PAD);
	let mut fills = Vec::new();
	let mut salt = 500u32;
	for aabb in colonnade_undercroft_aabbs(plan, keeps, y0, y1) {
		for chunk in split_aabb_along(aabb, COLONNADE_CHEST_CHUNK) {
			salt += 1;
			if let Some(fill) =
				chest_in_confines_clear(&Confines::from_bounds(chunk), noise, salt, &keep_outs)
			{
				fills.push(fill);
			}
		}
	}
	for aabb in keep_apron_aabbs(plan, keeps, y0, y1) {
		salt += 1;
		if let Some(fill) =
			chest_in_confines_clear(&Confines::from_bounds(aabb), noise, salt, &keep_outs)
		{
			fills.push(fill);
		}
	}
	fills
		.into_iter()
		.filter(|fill| chest_clear_of_shafts(fill.furniture.placement.translation, shafts))
		.map(|fill| fill.furniture)
		.collect()
}

fn colonnade_undercroft_aabbs(
	plan: &LesHallesFloorPlan,
	keeps: &[RingFortKeep],
	y0: f32,
	y1: f32,
) -> Vec<Aabb3d> {
	let cx = plan.center_xz.x;
	let cz = plan.center_xz.z;
	let ox = plan.outer.x * 0.5;
	let oz = plan.outer.y * 0.5;
	let ix = plan.courtyard.x * 0.5;
	let iz = plan.courtyard.y * 0.5;
	let keep = |sx: f32, sz: f32| {
		let i = CORNERS.iter().position(|&c| c == (sx, sz)).unwrap_or(0);
		&keeps[i]
	};
	let mut out = Vec::new();
	let mut push = |along_x: bool, a: &RingFortKeep, b: &RingFortKeep, inner: f32, outer: f32| {
		let along_center = if along_x { cx } else { cz };
		let along_inner_half = if along_x { ix } else { iz };
		let Some((start, end)) = run_between(a, b, along_x, along_center, along_inner_half) else {
			return;
		};
		let (inner, outer) = if inner < outer { (inner, outer) } else { (outer, inner) };
		out.push(if along_x {
			Aabb3d::from_min_max(Vec3::new(start, y0, inner), Vec3::new(end, y1, outer))
		} else {
			Aabb3d::from_min_max(Vec3::new(inner, y0, start), Vec3::new(outer, y1, end))
		});
	};
	push(true, keep(-1.0, 1.0), keep(1.0, 1.0), cz + iz + PILLAR_INSET, cz + oz - PILLAR_INSET);
	push(true, keep(-1.0, -1.0), keep(1.0, -1.0), cz - iz - PILLAR_INSET, cz - oz + PILLAR_INSET);
	push(false, keep(-1.0, -1.0), keep(-1.0, 1.0), cx - ix - PILLAR_INSET, cx - ox + PILLAR_INSET);
	push(false, keep(1.0, -1.0), keep(1.0, 1.0), cx + ix + PILLAR_INSET, cx + ox - PILLAR_INSET);
	out
}

fn keep_apron_aabbs(
	plan: &LesHallesFloorPlan,
	keeps: &[RingFortKeep],
	y0: f32,
	y1: f32,
) -> Vec<Aabb3d> {
	let mut out = Vec::new();
	for keep in keeps {
		let c = keep.center_xz();
		let half = keep.plan_half_extent();
		let toward_x = (plan.center_xz.x - c.x).signum();
		let toward_z = (plan.center_xz.z - c.z).signum();
		if toward_x.abs() < 0.5 || toward_z.abs() < 0.5 {
			continue;
		}
		let depth = KEEP_APRON_DEPTH;
		let along = KEEP_APRON_ALONG;
		let x0 = c.x + toward_x * half;
		let x1 = x0 + toward_x * depth;
		out.push(Aabb3d::from_min_max(
			Vec3::new(x0.min(x1), y0, c.z - along * 0.5),
			Vec3::new(x0.max(x1), y1, c.z + along * 0.5),
		));
		let z0 = c.z + toward_z * half;
		let z1 = z0 + toward_z * depth;
		out.push(Aabb3d::from_min_max(
			Vec3::new(c.x - along * 0.5, y0, z0.min(z1)),
			Vec3::new(c.x + along * 0.5, y1, z0.max(z1)),
		));
	}
	out
}

fn split_aabb_along(bounds: Aabb3d, chunk: f32) -> Vec<Aabb3d> {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let dx = (max.x - min.x).max(0.0);
	let dz = (max.z - min.z).max(0.0);
	let chunk = chunk.max(1.0);
	if dx >= dz {
		let n = ((dx / chunk).floor() as usize).max(1);
		let step = dx / n as f32;
		(0..n)
			.map(|i| {
				let x0 = min.x + i as f32 * step;
				Aabb3d::from_min_max(
					Vec3::new(x0, min.y, min.z),
					Vec3::new(x0 + step, max.y, max.z),
				)
			})
			.collect()
	} else {
		let n = ((dz / chunk).floor() as usize).max(1);
		let step = dz / n as f32;
		(0..n)
			.map(|i| {
				let z0 = min.z + i as f32 * step;
				Aabb3d::from_min_max(
					Vec3::new(min.x, min.y, z0),
					Vec3::new(max.x, max.y, z0 + step),
				)
			})
			.collect()
	}
}

fn chest_clear_of_shafts(p: Vec3, shafts: &[Aabb3d]) -> bool {
	!shafts.iter().any(|shaft| {
		p.x >= shaft.min.x && p.x <= shaft.max.x && p.z >= shaft.min.z && p.z <= shaft.max.z
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	fn fort_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-80.0, 0.0, -80.0), Vec3::new(80.0, 12.0, 80.0))
	}

	fn fit_fort(seed: i32) -> anyhow::Result<RingFortBuilding> {
		let confines = Confines::from_bounds(fort_bounds());
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		RingFortBuilding::fit_to_confines(&confines, noise)
			.map(|(fort, _)| fort)
			.map_err(|err| anyhow::anyhow!("ring fort fit failed: {err:?}"))
	}

	#[test]
	fn ring_is_two_to_four_storeys_with_four_taller_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(42)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let ring_floors = ring.tower.floor_count();
		anyhow::ensure!((2..=4).contains(&ring_floors), "ring storeys {ring_floors} outside 2..=4");
		let keeps: Vec<_> = fort.keeps().collect();
		anyhow::ensure!(keeps.len() == 4, "expected 4 corner keeps, got {}", keeps.len());
		for keep in &keeps {
			let n = keep.storey_count();
			anyhow::ensure!((5..=10).contains(&n), "keep storeys {n} outside 5..=10");
			anyhow::ensure!(n > ring_floors, "keep should out-climb the ring");
			anyhow::ensure!(!keep.stairwells().is_empty(), "keep needs stairwells");
		}
		anyhow::ensure!(fort.connected.edges.len() == 4);
		anyhow::ensure!(fort.connected.topology_is_valid());
		Ok(())
	}

	#[test]
	fn keeps_sit_on_the_gallery_not_off_the_corner() -> anyhow::Result<()> {
		let fort = fit_fort(11)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let gallery = last.parameterized.gallery_width;
		let hx = last.outer.x * 0.5 - gallery * 0.5;
		let hz = last.outer.y * 0.5 - gallery * 0.5;
		let eave_y = last.center_xz.y + last.storey_height;
		let outer_hx = last.outer.x * 0.5;
		let outer_hz = last.outer.y * 0.5;
		for keep in fort.keeps() {
			let c = keep.center_xz();
			let half = keep.plan_half_extent();
			let dx = (c.x - last.center_xz.x).abs();
			let dz = (c.z - last.center_xz.z).abs();
			anyhow::ensure!((dx - hx).abs() < 1e-3, "keep x {dx} vs gallery-corner {hx}");
			anyhow::ensure!((dz - hz).abs() < 1e-3, "keep z {dz} vs gallery-corner {hz}");
			anyhow::ensure!((c.y - eave_y).abs() < 1e-3, "keep y {} vs eave {eave_y}", c.y);
			anyhow::ensure!(
				(c.x - last.center_xz.x).abs() + half <= outer_hx + 0.25,
				"keep should stay on the outer footprint (x)"
			);
			anyhow::ensure!(
				(c.z - last.center_xz.z).abs() + half <= outer_hz + 0.25,
				"keep should stay on the outer footprint (z)"
			);
		}
		Ok(())
	}

	#[test]
	fn hosts_include_ring_keeps_stairs_and_gallery_roof() -> anyhow::Result<()> {
		let fort = fit_fort(7)?;
		let hosts = fort.hosts();
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::Ring(_))));
		anyhow::ensure!(hosts
			.iter()
			.any(|h| { matches!(h, RingFortHost::Circular(_) | RingFortHost::Trazaloid(_)) }));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::KeepStairwell(_))));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::Terrace(_))));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::TerraceStairwell(_))));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::GalleryColonnade(_))));
		anyhow::ensure!(hosts.iter().any(|h| matches!(h, RingFortHost::GalleryRoof(_))));
		anyhow::ensure!(!hosts.iter().any(|h| matches!(
			h,
			RingFortHost::Ring(inner) if matches!(inner.as_ref(), MixedUseLesHallesHost::Roof(_))
		)));
		Ok(())
	}

	#[test]
	fn terrace_packs_chests_under_the_colonnade() -> anyhow::Result<()> {
		let fort = fit_fort(7)?;
		let chests = fort.terrace.furniture_nodes_for_level(LodSceneLevel::High).flatten();
		anyhow::ensure!(!chests.is_empty(), "upper deck should keep residual chests");
		let eave_y = fort
			.ring()
			.and_then(|ring| ring.last_plan().cloned())
			.map(|plan| plan.center_xz.y + plan.storey_height)
			.ok_or_else(|| anyhow::anyhow!("missing last storey"))?;
		anyhow::ensure!(
			chests.iter().any(|node| (node.placement.translation.y - eave_y).abs() < 2.0),
			"chests should sit on the terrace deck"
		);
		Ok(())
	}

	fn xz_covers(volume: &Aabb3d, p: Vec3, pad: f32) -> bool {
		p.x > volume.min.x + pad
			&& p.x < volume.max.x - pad
			&& p.z > volume.min.z + pad
			&& p.z < volume.max.z - pad
	}

	#[test]
	fn ring_ceiling_has_terrace_stairs() -> anyhow::Result<()> {
		let fort = fit_fort(3)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		anyhow::ensure!(fort.terrace.deck().has_floor(), "colonnade needs a terrace deck");
		anyhow::ensure!(
			!last.gallery.has_ceiling(),
			"last gallery ceiling should yield to the terrace deck"
		);
		let eave_y = last.center_xz.y + last.storey_height;
		anyhow::ensure!(
			(fort.terrace.deck().params().center_xz.y - eave_y).abs() < 1e-3,
			"terrace should sit on the ring ceiling"
		);
		let balcony_z = last.center_xz.z + (last.gallery_inner.y + last.courtyard.y) * 0.25;
		anyhow::ensure!(
			fort.terrace.deck().floor_covers_xz(last.center_xz.x, balcony_z),
			"terrace should cover the balcony, not only the gallery band"
		);
		anyhow::ensure!(!fort.terrace_stairs.is_empty(), "stairs should climb onto the terrace");
		anyhow::ensure!(
			fort.terrace_stairs.len() == last.shaft_bounds.len(),
			"terrace stairs should continue the shafts below"
		);
		anyhow::ensure!(
			fort.terrace_stairs.iter().all(|s| (s.well().max().y - eave_y).abs() < 0.05),
			"terrace stairs should land on the ring ceiling"
		);
		Ok(())
	}

	#[test]
	fn gallery_roof_covers_ring_not_courtyard_or_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(19)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let volumes = &fort.roof.params().volumes;
		anyhow::ensure!(
			volumes.len() == 4,
			"expected 4 colonnade roof bars, got {}",
			volumes.len()
		);
		anyhow::ensure!(
			!fort.colonnade.pillars().is_empty(),
			"colonnade should stand on the terrace"
		);

		let court = last.center_xz;
		anyhow::ensure!(
			!volumes.iter().any(|v| xz_covers(v, court, 0.05)),
			"gallery roof covers courtyard center"
		);

		let gallery = Vec3::new(
			last.center_xz.x,
			0.0,
			last.center_xz.z + (last.outer.y + last.courtyard.y) * 0.25,
		);
		anyhow::ensure!(
			volumes.iter().any(|v| xz_covers(v, gallery, 0.05)),
			"north gallery should be roofed"
		);

		for keep in fort.keeps() {
			let c = keep.center_xz();
			let min_clear = keep.plan_half_extent() + TOWER_CLEAR;
			for v in volumes {
				let nearest_x = c.x.clamp(v.min.x, v.max.x);
				let nearest_z = c.z.clamp(v.min.z, v.max.z);
				let dx = c.x - nearest_x;
				let dz = c.z - nearest_z;
				let dist = (dx * dx + dz * dz).sqrt();
				anyhow::ensure!(
					dist + 1e-3 >= min_clear,
					"hip {dist:.1} too close to keep (need >= {min_clear:.1})"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn colonnade_rows_and_hips_share_the_run() -> anyhow::Result<()> {
		let fort = fit_fort(19)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing courtyard ring"))?;
		let last = ring.last_plan().ok_or_else(|| anyhow::anyhow!("ring has no storey"))?;
		let pillars = &fort.colonnade.pillars().pillars;
		anyhow::ensure!(!pillars.is_empty(), "expected colonnade piers");
		anyhow::ensure!(
			pillars.iter().all(|p| (p.height - PILLAR_HEIGHT).abs() < 1e-3),
			"loose / short piers in the colonnade"
		);
		anyhow::ensure!(pillars.len() % 2 == 0, "inner/outer rows should pair");
		let inner_n = pillars.len() / 2;
		anyhow::ensure!(inner_n >= 8, "expected paired piers on all four sides");

		for v in &fort.roof.params().volumes {
			let along_x = (v.max.x - v.min.x) >= (v.max.z - v.min.z);
			let (vmin, vmax) = if along_x { (v.min.x, v.max.x) } else { (v.min.z, v.max.z) };
			let (center, half) = if along_x {
				(last.center_xz.x, last.courtyard.x * 0.5)
			} else {
				(last.center_xz.z, last.courtyard.y * 0.5)
			};
			anyhow::ensure!(
				vmin + 1e-3 >= center - half + COLONNADE_INNER_END_INSET
					&& vmax <= center + half - COLONNADE_INNER_END_INSET + 1e-3,
				"hip {vmin:.1}..{vmax:.1} should stay inside inner span"
			);
			let hits = pillars.iter().any(|p| {
				let along = if along_x { p.center.x } else { p.center.z };
				(along - vmin).abs() < PILLAR_WIDTH || (along - vmax).abs() < PILLAR_WIDTH
			});
			anyhow::ensure!(hits, "hip {vmin:.1}..{vmax:.1} should end on a pier");
		}
		Ok(())
	}

	#[test]
	fn with_finish_shades_keeps_and_gallery_roof() -> anyhow::Result<()> {
		use building_components::BuildingComponents;
		use lod::scene::LodSceneLevel;
		use material_ref::MaterialId;

		let wall = MaterialRef::named("stucco");
		let roof = MaterialRef::named("iron");
		let mut saw_circular = false;
		let mut saw_trazaloid = false;
		for seed in 0..24 {
			let painted = fit_fort(seed)?.with_finish(wall.clone(), roof.clone());
			anyhow::ensure!(
				!painted.roof.roofs().is_empty()
					&& painted.roof.roofs().iter().all(|pitch| {
						matches!(
							pitch.surface_material().map(|m| &m.name),
							Some(MaterialId::Name(n)) if n == "iron"
						)
					}),
				"gallery pitches should carry the roof look"
			);
			let colonnade_nodes =
				painted.colonnade.panel_nodes_for_level(LodSceneLevel::High).flatten();
			anyhow::ensure!(
				colonnade_nodes.iter().any(|n| {
					matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
				}),
				"colonnade pillars should carry the wall look"
			);
			for keep in painted.keeps() {
				match keep {
					RingFortKeep::Circular(keep) => {
						saw_circular = true;
						let nodes =
							keep.shell.partition_nodes_for_level(LodSceneLevel::High).flatten();
						anyhow::ensure!(
							nodes.iter().any(|n| {
								matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
							}),
							"circular keep partitions should carry the wall look"
						);
					}
					RingFortKeep::Trazaloid(keep) => {
						saw_trazaloid = true;
						let nodes = keep.shell.panel_nodes_for_level(LodSceneLevel::High).flatten();
						anyhow::ensure!(
							nodes.iter().any(|n| {
								matches!(n.material.as_ref().map(|m| &m.name), Some(MaterialId::Name(n)) if n == "stucco")
							}),
							"trazaloid keep panels should carry the wall look"
						);
					}
				}
			}
			if saw_circular && saw_trazaloid {
				break;
			}
		}
		anyhow::ensure!(saw_circular, "expected a shaded circular keep");
		anyhow::ensure!(saw_trazaloid, "expected a shaded trazaloid keep");
		Ok(())
	}

	#[test]
	fn storey_counts_vary_across_seeds() -> anyhow::Result<()> {
		let mut ring_counts = std::collections::BTreeSet::new();
		let mut saw_circular = false;
		let mut saw_trazaloid = false;
		for seed in 0..24 {
			let fort = fit_fort(seed)?;
			let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
			ring_counts.insert(ring.tower.floor_count());
			for keep in fort.keeps() {
				match keep {
					RingFortKeep::Circular(_) => saw_circular = true,
					RingFortKeep::Trazaloid(_) => saw_trazaloid = true,
				}
			}
		}
		anyhow::ensure!(ring_counts.len() >= 2, "ring storeys should vary, got {ring_counts:?}");
		anyhow::ensure!(saw_circular, "expected at least one circular keep");
		anyhow::ensure!(saw_trazaloid, "expected at least one trazaloid keep");
		Ok(())
	}

	#[test]
	fn curtain_gallery_is_wide_enough_for_keeps() -> anyhow::Result<()> {
		let fort = fit_fort(5)?;
		let ring = fort.ring().ok_or_else(|| anyhow::anyhow!("missing ring"))?;
		let gallery = ring.gallery_width();
		anyhow::ensure!(gallery + 1e-3 >= 16.0, "curtain gallery {gallery:.1} too thin");
		for keep in fort.keeps() {
			anyhow::ensure!(
				keep.plan_half_extent() * 2.0 <= gallery + 0.5,
				"keep diameter {} vs gallery {gallery}",
				keep.plan_half_extent() * 2.0
			);
		}
		Ok(())
	}
}
