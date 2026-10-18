//! Livable gallery usage: lengthwise RLA bays on ExternalSpace strips.
//!
//! Courtyard / balcony [`OpeningLabel::Passage`] doors are the RLA passages —
//! no [`crate::HallsToShafts`], no suite packing, no [`crate::LivableApartment`]
//! entry carve. SpineHall stays off this path (SingleClosed → Guillotine →
//! AllOpen). Party walls and RLA internals use the same
//! [`INTERNAL_WALLS_LAYER`] High-only band as [`crate::LivableApartments`].

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::{PanelNode, PanelStyle};
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{
	aabb_xz_extent, aabb_xz_overlap_area, Confines, FillRegion, FillableRegions, FitError,
	SpaceKind,
};
use crate::openings::{OpeningId, OpeningLabel, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::rectangular_strip::RectangularStripNode;
use crate::paneling::DEFAULT_PANEL_THICKNESS;
use crate::usage_areas::furniture_util::{
	as_closet_if_internal, chests_for_regions, FurnitureFill,
};
use crate::usage_areas::livable_apartment::INTERNAL_WALLS_LAYER;
use crate::usage_areas::plan_access::DEFAULT_WALK_CLEAR;
use crate::usage_areas::plan_cells::shared_edge_span;
use crate::usage_areas::plan_geom::{host_xz, noise_for_cell};
use crate::usage_areas::rectangular_livable_area::{
	RectLivableStrategy, RectQuarterKind, RectangularLivableArea,
	RectangularLivableAreaParameterized, DEFAULT_CLOSED_MAX_AREA,
};

use super::LesHallesUsagePlan;

const EPS: f32 = 1e-3;
/// Soft-fail strips shorter than this along the long axis.
const MIN_STRIP_ALONG: f32 = 3.5;
/// Prefer livable bays at least this long (m) before merging voronoi cells.
const MIN_BAY_ALONG: f32 = 5.0;
const MAX_BAY_ALONG: f32 = 14.0;
/// Area used when converting strip depth → preferred along length (m²).
const TARGET_BAY_AREA: f32 = 32.0;
const SALT_BAY_ALONG: f32 = 121.0;
const SALT_CROSS_STRIP_WALL: f32 = 131.0;
/// Minimum shared-edge length (m) before a cross-strip party wall is considered.
const MIN_CROSS_STRIP_SPAN: f32 = 2.0;
/// Bedroom enters the multi-room program from this footprint (m²).
const BEDROOM_PROGRAM_AREA: f32 = 12.0;

/// Residential gallery fill: lengthwise RLA bays + party walls.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesLivableUsage {
	/// One RLA per filled gallery bay (all strips flattened).
	pub areas: Vec<RectangularLivableArea>,
	/// Within-strip bay cuts + noisy cross-strip shared edges.
	pub party_walls: Vec<ClippedRectangularStrip>,
	/// Chests in leftover gallery / closet pockets the RLA packer could not fill.
	pub residual_chests: Vec<FurnitureFill>,
}

impl LesHallesUsagePlan for LesHallesLivableUsage {
	fn paint(
		regions: FillableRegions,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let mut tagged: Vec<(i32, RectangularLivableArea)> = Vec::new();
		let mut party_walls = Vec::new();
		let mut residual_within = Vec::new();
		let mut strip_i = 0i32;

		for region in regions.within {
			if region.kind != SpaceKind::ExternalSpace {
				residual_within.push(region);
				continue;
			}
			let strip_noise = noise_for_cell(noise, strip_i);
			match fill_strip_bays(&region.confines, strip_noise) {
				Ok((filled, walls, nested)) => {
					for area in filled {
						tagged.push((strip_i, area));
					}
					party_walls.extend(walls);
					residual_within.extend(nested.within.into_iter().map(as_closet_if_internal));
				}
				Err(FitError::TooSmall { reason }) if reason == "no passage" => {
					match fit_open_strip_without_passages(&region.confines, strip_noise) {
						Ok((area, nested)) => {
							tagged.push((strip_i, area));
							residual_within
								.extend(nested.within.into_iter().map(as_closet_if_internal));
						}
						Err(FitError::TooSmall { .. }) => {
							residual_within
								.push(FillRegion::new(SpaceKind::ExternalSpace, region.confines));
						}
						Err(err) => return Err(err),
					}
				}
				Err(FitError::TooSmall { .. }) => {
					residual_within
						.push(FillRegion::new(SpaceKind::ExternalSpace, region.confines));
				}
				Err(err) => return Err(err),
			}
			strip_i += 1;
		}

		party_walls.extend(noisy_cross_strip_party_walls(&tagged, noise));
		let areas = tagged.into_iter().map(|(_, a)| a).collect();
		let residual_chests = chests_for_regions(&residual_within, noise);

		Ok((
			Self { areas, party_walls, residual_chests },
			FillableRegions { within: residual_within, atop: regions.atop },
		))
	}
}

impl BuildingComponents for LesHallesLivableUsage {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.party_walls {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.panel_nodes_for_level(level));
		}
		for area in &self.areas {
			out.extend_under(INTERNAL_WALLS_LAYER, area.panel_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for wall in &self.party_walls {
			out.extend_under(INTERNAL_WALLS_LAYER, wall.joint_nodes_for_level(level));
		}
		for area in &self.areas {
			out.extend_under(INTERNAL_WALLS_LAYER, area.joint_nodes_for_level(level));
		}
		structural_layers(level, out)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for area in &self.areas {
			out.extend(area.furniture_nodes_for_level(level));
		}
		out.extend(Layers::from_free(
			self.residual_chests.iter().map(|fill| fill.furniture.clone()).collect(),
		));
		out
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		let mut out = Layers::new();
		for area in &self.areas {
			out.extend(area.furniture_usage_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for area in &self.areas {
			out.extend(area.label_nodes_for_level(level));
		}
		out.extend(Layers::from_free(
			self.residual_chests.iter().map(|fill| fill.label.clone()).collect(),
		));
		out
	}
}

fn fit_open_strip_without_passages(
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	RectangularLivableArea::fit_with_params(
		confines,
		noise,
		rla_params(RectLivableStrategy::AllOpen),
		&[RectQuarterKind::Living],
	)
}

/// High keeps tagged internals; coarser bands drop [`INTERNAL_WALLS_LAYER`].
fn structural_layers<T>(level: LodSceneLevel, layers: Layers<T>) -> Layers<T> {
	if matches!(level, LodSceneLevel::High) {
		layers
	} else {
		layers.except([INTERNAL_WALLS_LAYER])
	}
}

#[derive(Debug, Clone)]
struct PassageAlong {
	id: OpeningId,
	center: f32,
}

#[derive(Debug, Clone)]
struct LivableBay {
	along0: f32,
	along1: f32,
	passage_ids: Vec<OpeningId>,
}

impl LivableBay {
	fn merge_with(&mut self, other: LivableBay) {
		self.along0 = self.along0.min(other.along0);
		self.along1 = self.along1.max(other.along1);
		self.passage_ids.extend(other.passage_ids);
	}
}

fn fill_strip_bays(
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(Vec<RectangularLivableArea>, Vec<ClippedRectangularStrip>, FillableRegions), FitError>
{
	let min = Vec3::from(confines.bounds.min);
	let max = Vec3::from(confines.bounds.max);
	let extent = aabb_xz_extent(&confines.bounds);
	let height = (max.y - min.y).max(1e-4);
	if height < 2.0 {
		return Err(FitError::TooSmall { reason: "height" });
	}
	let along_x = extent.x >= extent.y;
	let along = if along_x { extent.x } else { extent.y };
	let depth = if along_x { extent.y } else { extent.x };
	if along < MIN_STRIP_ALONG || depth < 2.5 {
		return Err(FitError::TooSmall { reason: "strip" });
	}

	let passages = collect_passages_along(&confines.openings, along_x, min, along);
	if passages.is_empty() {
		return Err(FitError::TooSmall { reason: "no passage" });
	}

	let min_bay = sample_min_bay_along(confines, noise, depth, along);
	let bays = partition_bays_for_passages(&passages, along, min_bay);
	fit_areas_covering_strip(confines, along_x, min, max, &bays, noise)
}

fn sample_min_bay_along(confines: &Confines, noise: NoiseParams, depth: f32, along: f32) -> f32 {
	let area_driven = (TARGET_BAY_AREA / depth.max(EPS)).clamp(MIN_BAY_ALONG, MAX_BAY_ALONG);
	let cfg = NoiseConfig::new(noise);
	let c = confines.center();
	let sampled =
		cfg.sample_range_f32_4d(MIN_BAY_ALONG, MAX_BAY_ALONG, c.x, c.y, c.z, SALT_BAY_ALONG);
	sampled.max(area_driven).clamp(MIN_BAY_ALONG, along.max(MIN_BAY_ALONG))
}

fn rla_params(strategy: RectLivableStrategy) -> RectangularLivableAreaParameterized {
	RectangularLivableAreaParameterized {
		strategy,
		min_hall: DEFAULT_WALK_CLEAR,
		closed_max_area: DEFAULT_CLOSED_MAX_AREA,
	}
}

/// SpineHall stays off this typology — closed studios first, then guillotine.
fn les_halles_strategies(area_m2: f32, passages: usize) -> Vec<RectLivableStrategy> {
	if passages == 1 && area_m2 + EPS <= DEFAULT_CLOSED_MAX_AREA {
		vec![
			RectLivableStrategy::SingleClosed,
			RectLivableStrategy::GuillotineSplit,
			RectLivableStrategy::AllOpen,
		]
	} else {
		vec![
			RectLivableStrategy::GuillotineSplit,
			RectLivableStrategy::SingleClosed,
			RectLivableStrategy::AllOpen,
		]
	}
}

fn fit_areas_covering_strip(
	confines: &Confines,
	along_x: bool,
	strip_min: Vec3,
	strip_max: Vec3,
	bays: &[LivableBay],
	noise: NoiseParams,
) -> Result<(Vec<RectangularLivableArea>, Vec<ClippedRectangularStrip>, FillableRegions), FitError>
{
	debug_assert!(!bays.is_empty());

	let mut fitted: Vec<(LivableBay, RectangularLivableArea)> = Vec::new();
	let mut residual_within = Vec::new();
	let mut carry: Option<LivableBay> = None;

	for (i, bay) in bays.iter().cloned().enumerate() {
		let bay = match carry.take() {
			Some(mut pending) => {
				pending.merge_with(bay);
				pending
			}
			None => bay,
		};

		match fit_rla_bay(confines, along_x, strip_min, strip_max, &bay, noise, i) {
			Ok((area, nested)) => {
				residual_within.extend(nested.within);
				fitted.push((bay, area));
			}
			Err(FitError::TooSmall { .. }) => {
				if let Some((prev_bay, prev_area)) = fitted.last_mut() {
					prev_bay.merge_with(bay);
					let (area, nested) =
						fit_rla_bay(confines, along_x, strip_min, strip_max, prev_bay, noise, i)
							.map_err(|_| FitError::TooSmall { reason: "areas cover" })?;
					residual_within.extend(nested.within);
					*prev_area = area;
				} else {
					carry = Some(bay);
				}
			}
			Err(err) => return Err(err),
		}
	}

	if let Some(tail) = carry {
		if let Some((prev_bay, prev_area)) = fitted.last_mut() {
			prev_bay.merge_with(tail);
			let (area, nested) =
				fit_rla_bay(confines, along_x, strip_min, strip_max, prev_bay, noise, 0)
					.map_err(|_| FitError::TooSmall { reason: "areas cover" })?;
			residual_within.extend(nested.within);
			*prev_area = area;
		} else {
			let (area, nested) =
				fit_rla_bay(confines, along_x, strip_min, strip_max, &tail, noise, 0)?;
			residual_within.extend(nested.within);
			fitted.push((tail, area));
		}
	}

	if fitted.is_empty() {
		return Err(FitError::TooSmall { reason: "areas" });
	}

	let mut party_walls = Vec::new();
	for window in fitted.windows(2) {
		let cut = window[0].0.along1;
		if let Some(wall) = party_wall_at_cut(along_x, cut, strip_min, strip_max) {
			party_walls.push(wall);
		}
	}

	Ok((
		fitted.into_iter().map(|(_, area)| area).collect(),
		party_walls,
		FillableRegions { within: residual_within, atop: Vec::new() },
	))
}

fn fit_rla_bay(
	confines: &Confines,
	along_x: bool,
	strip_min: Vec3,
	strip_max: Vec3,
	bay: &LivableBay,
	noise: NoiseParams,
	seed_i: usize,
) -> Result<(RectangularLivableArea, FillableRegions), FitError> {
	let (smin, smax) = if along_x {
		(
			Vec3::new(strip_min.x + bay.along0, strip_min.y, strip_min.z),
			Vec3::new(strip_min.x + bay.along1, strip_max.y, strip_max.z),
		)
	} else {
		(
			Vec3::new(strip_min.x, strip_min.y, strip_min.z + bay.along0),
			Vec3::new(strip_max.x, strip_max.y, strip_min.z + bay.along1),
		)
	};
	let bay_bounds = Aabb3d::from_min_max(smin, smax);
	let cell = Confines::new(
		bay_bounds,
		confines.roll,
		openings_for_bay(&confines.openings, &bay_bounds, &bay.passage_ids),
	);
	let mut bay_noise = noise;
	bay_noise.seed = noise.seed.wrapping_add(seed_i as i32 * 17);
	let area_m2 = {
		let fp = cell.footprint();
		fp.x * fp.y
	};
	let passages = bay.passage_ids.len();
	let mut last = FitError::TooSmall { reason: "rla_bay_exhausted" };
	for strategy in les_halles_strategies(area_m2, passages) {
		let program = bay_program_for_strategy(area_m2, passages, strategy);
		match RectangularLivableArea::fit_with_params(
			&cell,
			bay_noise,
			rla_params(strategy),
			&program,
		) {
			Ok(ok) => return Ok(ok),
			Err(FitError::TooSmall { reason }) => {
				last = FitError::TooSmall { reason };
			}
			Err(err) => return Err(err),
		}
	}
	Err(last)
}

/// Studio closed-only for SingleClosed; otherwise bedroom-first multi-room.
///
/// Bedroom used to require `area > 28` while target bay area was ~28 m², so most
/// multi-room programs omitted it. Guillotine-only also failed many closed-only
/// studio programs that SingleClosed handles.
fn bay_program_for_strategy(
	area: f32,
	_passages: usize,
	strategy: RectLivableStrategy,
) -> Vec<RectQuarterKind> {
	if matches!(strategy, RectLivableStrategy::SingleClosed) {
		return vec![RectQuarterKind::Bedroom];
	}
	let mut out = Vec::new();
	// Bedroom first so Guillotine's alternating split lands a closed cell early.
	if area + EPS >= BEDROOM_PROGRAM_AREA {
		out.push(RectQuarterKind::Bedroom);
	}
	if area + EPS >= 12.0 {
		out.push(RectQuarterKind::Eating);
	}
	out.push(RectQuarterKind::Living);
	if area > 40.0 {
		out.push(RectQuarterKind::Bathroom);
	}
	out
}

fn collect_passages_along(
	openings: &Openings,
	along_x: bool,
	strip_min: Vec3,
	along: f32,
) -> Vec<PassageAlong> {
	let origin = if along_x { strip_min.x } else { strip_min.z };
	let mut out = Vec::new();
	for (id, opening) in openings.iter() {
		if !matches!(opening.label, OpeningLabel::Passage) {
			continue;
		}
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		let c = if along_x {
			(omin.x + omax.x) * 0.5 - origin
		} else {
			(omin.z + omax.z) * 0.5 - origin
		};
		if c < -0.5 || c > along + 0.5 {
			continue;
		}
		out.push(PassageAlong { id: id.clone(), center: c.clamp(0.0, along) });
	}
	out.sort_by(|a, b| {
		a.center
			.partial_cmp(&b.center)
			.unwrap_or(std::cmp::Ordering::Equal)
			.then_with(|| a.id.as_str().cmp(b.id.as_str()))
	});
	out
}

/// Voronoi partition of `[0, along]` by passage centers, then merge short cells.
fn partition_bays_for_passages(
	passages: &[PassageAlong],
	along: f32,
	min_bay: f32,
) -> Vec<LivableBay> {
	debug_assert!(!passages.is_empty());
	let n = passages.len();
	let mut edges = Vec::with_capacity(n + 1);
	edges.push(0.0);
	for i in 0..n.saturating_sub(1) {
		edges.push((passages[i].center + passages[i + 1].center) * 0.5);
	}
	edges.push(along);

	let mut bays: Vec<LivableBay> = (0..n)
		.map(|i| LivableBay {
			along0: edges[i],
			along1: edges[i + 1],
			passage_ids: vec![passages[i].id.clone()],
		})
		.collect();

	let mut i = 0;
	while i < bays.len() {
		let w = bays[i].along1 - bays[i].along0;
		if w + 1e-4 >= min_bay || bays.len() == 1 {
			i += 1;
			continue;
		}
		if i + 1 < bays.len() {
			let right = bays.remove(i + 1);
			bays[i].along1 = right.along1;
			bays[i].passage_ids.extend(right.passage_ids);
		} else if i > 0 {
			let cur = bays.remove(i);
			let prev = &mut bays[i - 1];
			prev.along1 = cur.along1;
			prev.passage_ids.extend(cur.passage_ids);
		} else {
			break;
		}
	}
	bays
}

fn openings_for_bay(
	openings: &Openings,
	bounds: &Aabb3d,
	owned_passages: &[OpeningId],
) -> Openings {
	let region = Aabb2d {
		min: Vec2::new(Vec3::from(bounds.min).x, Vec3::from(bounds.min).z),
		max: Vec2::new(Vec3::from(bounds.max).x, Vec3::from(bounds.max).z),
	};
	let y0 = Vec3::from(bounds.min).y;
	let y1 = Vec3::from(bounds.max).y;
	let mut out = Openings::new();
	for id in owned_passages {
		if let Some(opening) = openings.get(id) {
			out.insert(id.clone(), opening.clone());
		}
	}
	for (id, opening) in openings.iter() {
		if matches!(opening.label, OpeningLabel::Passage | OpeningLabel::Shaft) {
			continue;
		}
		if aabb_xz_overlap_area(&opening.bounds, &region) <= 1e-4 {
			continue;
		}
		let omin = Vec3::from(opening.bounds.min);
		let omax = Vec3::from(opening.bounds.max);
		if omax.y < y0 - 1e-3 || omin.y > y1 + 1e-3 {
			continue;
		}
		out.insert(id.clone(), opening.clone());
	}
	out
}

fn party_wall_at_cut(
	along_x: bool,
	cut: f32,
	strip_min: Vec3,
	strip_max: Vec3,
) -> Option<ClippedRectangularStrip> {
	let height = strip_max.y - strip_min.y;
	if height < EPS {
		return None;
	}
	let (start, end) = if along_x {
		(
			Vec3::new(strip_min.x + cut, strip_min.y, strip_min.z),
			Vec3::new(strip_min.x + cut, strip_min.y, strip_max.z),
		)
	} else {
		(
			Vec3::new(strip_min.x, strip_min.y, strip_min.z + cut),
			Vec3::new(strip_max.x, strip_min.y, strip_min.z + cut),
		)
	};
	solid_party_wall(start, end, height)
}

/// Optional party walls on shared edges between bays from different strips.
///
/// Leaving a boundary open yields L-shaped living across the corner; walling
/// it separates the units. Choice is per shared span from spatial noise.
fn noisy_cross_strip_party_walls(
	tagged: &[(i32, RectangularLivableArea)],
	noise: NoiseParams,
) -> Vec<ClippedRectangularStrip> {
	let cfg = NoiseConfig::new(noise);
	let mut walls = Vec::new();
	for i in 0..tagged.len() {
		for j in (i + 1)..tagged.len() {
			let (si, a) = &tagged[i];
			let (sj, b) = &tagged[j];
			if si == sj {
				continue;
			}
			let a_xz = host_xz(&a.confines.bounds);
			let b_xz = host_xz(&b.confines.bounds);
			let Some((along_x, lo, hi, mid)) = shared_edge_span(a_xz, b_xz) else {
				continue;
			};
			if hi - lo + EPS < MIN_CROSS_STRIP_SPAN {
				continue;
			}
			let y0 = a.confines.bounds.min.y.max(b.confines.bounds.min.y);
			let y1 = a.confines.bounds.max.y.min(b.confines.bounds.max.y);
			let height = y1 - y0;
			if height < EPS {
				continue;
			}
			let sample = cfg.sample_range_f32_4d(
				0.0,
				1.0,
				mid,
				0.5 * (lo + hi),
				y0,
				SALT_CROSS_STRIP_WALL + (*si as f32) * 3.0 + *sj as f32,
			);
			if sample >= 0.5 {
				continue;
			}
			let (start, end) = if along_x {
				(Vec3::new(lo, y0, mid), Vec3::new(hi, y0, mid))
			} else {
				(Vec3::new(mid, y0, lo), Vec3::new(mid, y0, hi))
			};
			if let Some(wall) = solid_party_wall(start, end, height) {
				walls.push(wall);
			}
		}
	}
	walls
}

fn solid_party_wall(start: Vec3, end: Vec3, height: f32) -> Option<ClippedRectangularStrip> {
	if start.distance(end) < EPS || height < EPS {
		return None;
	}
	let thickness = DEFAULT_PANEL_THICKNESS.max(0.12);
	Some(ClippedRectangularStrip::from_nodes(
		PanelStyle::RoughStonework,
		[
			RectangularStripNode::new(start, height, thickness, 0.0),
			RectangularStripNode::new(end, height, thickness, 0.0),
		],
		[None],
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn partition_merges_short_voronoi_cells() {
		let passages = vec![
			PassageAlong { id: OpeningId::new("a"), center: 2.0 },
			PassageAlong { id: OpeningId::new("b"), center: 4.0 },
			PassageAlong { id: OpeningId::new("c"), center: 12.0 },
		];
		let bays = partition_bays_for_passages(&passages, 16.0, 5.0);
		assert!(bays.len() < 3);
		assert!((bays.first().unwrap().along0).abs() < EPS);
		assert!((bays.last().unwrap().along1 - 16.0).abs() < EPS);
	}
}
