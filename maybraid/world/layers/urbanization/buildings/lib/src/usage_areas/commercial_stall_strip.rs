//! Strip of commercial stalls along a gallery band.
//!
//! **Semantically:** the Les Halles (or similar) gallery shop front — many bays
//! along an external strip, each with its own door(s) onto the walkway.
//!
//! **Programmatically:** voronoi bays along the strip long axis from
//! [`OpeningLabel::Passage`] openings (merge undersized cells), subset openings
//! per bay, then [`CommercialStall::fit_to_confines`] each bay. Every fitted
//! stall uniquely owns ≥1 passage; runs that cannot host their own stall
//! (including leading/trailing corner overhangs past the last door, or a
//! `TooSmall` bay) are absorbed by extending the previous stall’s bounds.
//! Soft-fail the whole strip if it is shorter than the minimum along length.
//! A strip with no passages still gets one lounge stall so the bay is not a void.

pub mod commercial_stall;

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use bevy_math::Vec3;
use building_components::furniture::FurnitureNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{aabb_xz_extent, aabb_xz_overlap_area, Confines, FillableRegions, Fit, FitError};
use crate::openings::{OpeningId, OpeningLabel, Openings};

pub use commercial_stall::{
	BitesSitdownStall, BitesStall, CommercialStall, CommercialStallInterior,
	CommercialStallParameterized, CommercialStallPlan, KnickKnackStall,
	KnickKnackStallParameterized, KnickKnackStallPlan, MiniMart, PartsStall, PublicRestroom,
	PublicRestroomParameterized, PublicRestroomPlan,
};

/// Prefer larger gallery bays; merge voronoi cells below this when sampled.
const MIN_STALL_ALONG: f32 = 3.5;
const MAX_STALL_ALONG: f32 = 9.0;
const MIN_STRIP_ALONG: f32 = 3.5;

/// Noise knobs for packing stalls along a strip.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommercialStallStripParameterized {
	/// Minimum preferred bay width along the strip’s long axis (merge threshold).
	pub bay_width: f32,
}

impl CommercialStallStripParameterized {
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Self {
		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let bay = cfg.sample_range_f32_4d(MIN_STALL_ALONG, MAX_STALL_ALONG, c.x, c.y, c.z, 21.0);
		Self { bay_width: bay }
	}
}

/// Strip plan: ordered stall cells along the long plan axis.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStripPlan {
	pub parameterized: CommercialStallStripParameterized,
	pub stalls: Vec<CommercialStall>,
}

impl CommercialStallStripPlan {
	pub fn from_parameterized(
		params: CommercialStallStripParameterized,
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<Self, FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let extent = aabb_xz_extent(&confines.bounds);
		let height = (max.y - min.y).max(1e-4);
		if height < 0.4 {
			return Err(FitError::TooSmall { reason: "height" });
		}
		let along_x = extent.x >= extent.y;
		let along = if along_x { extent.x } else { extent.y };
		let depth = if along_x { extent.y } else { extent.x };
		if along < MIN_STRIP_ALONG || depth < 0.4 {
			return Err(FitError::TooSmall { reason: "strip" });
		}

		let passages = collect_passages_along(&confines.openings, along_x, min, along);
		if passages.is_empty() {
			let (stall, _) = CommercialStall::fit_to_confines(confines, noise)?;
			return Ok(Self { parameterized: params, stalls: vec![stall] });
		}

		let min_bay = params.bay_width.clamp(MIN_STALL_ALONG, along.max(MIN_STALL_ALONG));
		let bays = partition_bays_for_passages(&passages, along, min_bay);
		let stalls = fit_stalls_covering_strip(confines, along_x, min, max, &bays, noise)?;
		Ok(Self { parameterized: params, stalls })
	}
}

#[derive(Debug, Clone)]
struct PassageAlong {
	id: OpeningId,
	/// Center of the opening along the strip axis, relative to strip start.
	center: f32,
}

#[derive(Debug, Clone)]
struct StallBay {
	along0: f32,
	along1: f32,
	passage_ids: Vec<OpeningId>,
}

impl StallBay {
	fn merge_with(&mut self, other: StallBay) {
		self.along0 = self.along0.min(other.along0);
		self.along1 = self.along1.max(other.along1);
		self.passage_ids.extend(other.passage_ids);
	}
}

/// Fit each passage bay; never leave an uncovered along-run.
///
/// On [`FitError::TooSmall`], absorb the bay into the previous fitted stall
/// (extend bounds + passages and re-fit). A leading failure is carried into the
/// next bay. Every returned stall still owns ≥1 passage uniquely.
fn fit_stalls_covering_strip(
	confines: &Confines,
	along_x: bool,
	strip_min: Vec3,
	strip_max: Vec3,
	bays: &[StallBay],
	noise: NoiseParams,
) -> Result<Vec<CommercialStall>, FitError> {
	debug_assert!(!bays.is_empty());
	debug_assert!(bays.iter().all(|b| !b.passage_ids.is_empty()));

	let mut fitted: Vec<(StallBay, CommercialStall, usize)> = Vec::new();
	let mut carry: Option<StallBay> = None;

	for (i, bay) in bays.iter().cloned().enumerate() {
		let bay = match carry.take() {
			Some(mut pending) => {
				pending.merge_with(bay);
				pending
			}
			None => bay,
		};
		debug_assert!(!bay.passage_ids.is_empty());

		match fit_stall_bay(confines, along_x, strip_min, strip_max, &bay, noise, i) {
			Ok(stall) => fitted.push((bay, stall, i)),
			Err(FitError::TooSmall { .. }) => {
				if let Some((prev_bay, prev_stall, prev_i)) = fitted.last_mut() {
					// Orphan / failed run: extend the last stall over it.
					prev_bay.merge_with(bay);
					*prev_stall = fit_stall_bay(
						confines, along_x, strip_min, strip_max, prev_bay, noise, *prev_i,
					)
					.map_err(|_| FitError::TooSmall { reason: "stalls cover" })?;
				} else {
					carry = Some(bay);
				}
			}
			Err(err) => return Err(err),
		}
	}

	if let Some(tail) = carry {
		if let Some((prev_bay, prev_stall, prev_i)) = fitted.last_mut() {
			prev_bay.merge_with(tail);
			*prev_stall =
				fit_stall_bay(confines, along_x, strip_min, strip_max, prev_bay, noise, *prev_i)
					.map_err(|_| FitError::TooSmall { reason: "stalls cover" })?;
		} else {
			// Single merged bay for the whole strip.
			let stall = fit_stall_bay(confines, along_x, strip_min, strip_max, &tail, noise, 0)?;
			fitted.push((tail, stall, 0));
		}
	}

	if fitted.is_empty() {
		return Err(FitError::TooSmall { reason: "stalls" });
	}

	debug_assert!(fitted.first().unwrap().0.along0.abs() < 1e-2);
	for w in fitted.windows(2) {
		debug_assert!((w[0].0.along1 - w[1].0.along0).abs() < 1e-2);
	}

	Ok(fitted.into_iter().map(|(_, stall, _)| stall).collect())
}

fn fit_stall_bay(
	confines: &Confines,
	along_x: bool,
	strip_min: Vec3,
	strip_max: Vec3,
	bay: &StallBay,
	noise: NoiseParams,
	seed_i: usize,
) -> Result<CommercialStall, FitError> {
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
	CommercialStall::fit_to_confines(&cell, bay_noise).map(|(stall, _)| stall)
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
) -> Vec<StallBay> {
	debug_assert!(!passages.is_empty());
	let n = passages.len();
	let mut edges = Vec::with_capacity(n + 1);
	edges.push(0.0);
	for i in 0..n.saturating_sub(1) {
		edges.push((passages[i].center + passages[i + 1].center) * 0.5);
	}
	edges.push(along);

	let mut bays: Vec<StallBay> = (0..n)
		.map(|i| StallBay {
			along0: edges[i],
			along1: edges[i + 1],
			passage_ids: vec![passages[i].id.clone()],
		})
		.collect();

	// Merge left→right while a bay is shorter than the preferred minimum.
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

/// Owned passages (unique) plus intersecting non-passage opens for the bay.
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

/// Full commercial stall strip.
#[derive(Debug, Clone, PartialEq)]
pub struct CommercialStallStrip {
	pub plan: CommercialStallStripPlan,
}

impl CommercialStallStrip {
	pub fn from_plan(plan: CommercialStallStripPlan) -> Self {
		Self { plan }
	}

	pub fn stalls(&self) -> &[CommercialStall] {
		&self.plan.stalls
	}
}

impl Fit for CommercialStallStrip {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommercialStallStripParameterized::sample(confines, noise);
		let plan = CommercialStallStripPlan::from_parameterized(params, confines, noise)?;
		Ok((Self::from_plan(plan), FillableRegions::empty()))
	}
}

impl BuildingComponents for CommercialStallStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.label_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.furniture_nodes_for_level(level));
		}
		out
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		let mut out = Layers::new();
		for stall in &self.plan.stalls {
			out.extend(stall.furniture_usage_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::Opening;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	fn strip_with_doors(door_mids: &[f32], extent_x: f32) -> Confines {
		let mut openings = Openings::new();
		for (i, &mid) in door_mids.iter().enumerate() {
			openings.insert(
				OpeningId::new(format!("door_{i}")),
				Opening::passage(Aabb3d::from_min_max(
					Vec3::new(mid - 0.6, 0.0, -0.2),
					Vec3::new(mid + 0.6, 2.2, 0.2),
				)),
			);
		}
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(extent_x, 3.5, 5.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn strip_packs_one_stall_per_passage() {
		let confines = strip_with_doors(&[2.5, 7.5, 12.5], 15.0);
		// Explicit min bay below voronoi cell width (~5m) so cells do not merge.
		let params = CommercialStallStripParameterized { bay_width: 3.5 };
		let plan =
			CommercialStallStripPlan::from_parameterized(params, &confines, NoiseParams::default())
				.unwrap();
		assert_eq!(plan.stalls.len(), 3);
		let strip = CommercialStallStrip::from_plan(plan);
		assert!(!strip.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
		assert!(!strip.panel_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn each_bay_owns_passages_uniquely() {
		let confines = strip_with_doors(&[3.0, 9.0, 15.0], 18.0);
		let passages = collect_passages_along(&confines.openings, true, Vec3::ZERO, 18.0);
		let bays = partition_bays_for_passages(&passages, 18.0, 3.5);
		assert_eq!(bays.len(), 3);
		let mut seen = std::collections::HashSet::new();
		for bay in &bays {
			assert!(!bay.passage_ids.is_empty());
			for id in &bay.passage_ids {
				assert!(seen.insert(id.clone()), "passage {} claimed twice", id.as_str());
			}
			let bay_bounds = Aabb3d::from_min_max(
				Vec3::new(bay.along0, 0.0, 0.0),
				Vec3::new(bay.along1, 3.5, 5.0),
			);
			let cell_openings = openings_for_bay(&confines.openings, &bay_bounds, &bay.passage_ids);
			let cell_passages: std::collections::HashSet<_> = cell_openings
				.iter()
				.filter(|(_, o)| matches!(o.label, OpeningLabel::Passage))
				.map(|(id, _)| id.clone())
				.collect();
			let owned: std::collections::HashSet<_> = bay.passage_ids.iter().cloned().collect();
			assert_eq!(cell_passages, owned);
		}
	}

	#[test]
	fn close_passages_merge_into_larger_stall() {
		// Three doors clustered; min bay 6m on a 12m strip → fewer than 3 stalls.
		let confines = strip_with_doors(&[2.0, 3.5, 5.0], 12.0);
		let params = CommercialStallStripParameterized { bay_width: 6.0 };
		let plan =
			CommercialStallStripPlan::from_parameterized(params, &confines, NoiseParams::default())
				.unwrap();
		assert!(plan.stalls.len() < 3);
		assert!(!plan.stalls.is_empty());
	}

	#[test]
	fn without_passage_strip_gets_a_lounge() {
		let confines =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.5, 5.0)));
		let (strip, _) =
			CommercialStallStrip::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert_eq!(strip.stalls().len(), 1);
		assert!(!strip.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
		assert!(
			strip
				.furniture_nodes_for_level(LodSceneLevel::High)
				.flatten()
				.iter()
				.any(|node| node.geometry == building_components::FurnitureGeometry::Chair),
			"lounge sitting should reach the strip chair collection"
		);
	}

	#[test]
	fn partition_covers_full_strip_including_corner_overhangs() {
		// Les Halles-style: doors only on the inner straight run; corner squares
		// beyond the first/last door still belong to the end bays.
		let confines = strip_with_doors(&[10.0, 20.0, 30.0], 40.0);
		let passages = collect_passages_along(&confines.openings, true, Vec3::ZERO, 40.0);
		let bays = partition_bays_for_passages(&passages, 40.0, 3.5);
		assert!(bays.first().unwrap().along0.abs() < 1e-3);
		assert!((bays.last().unwrap().along1 - 40.0).abs() < 1e-3);
		for bay in &bays {
			assert!(!bay.passage_ids.is_empty());
		}
		for w in bays.windows(2) {
			assert!((w[0].along1 - w[1].along0).abs() < 1e-3);
		}
	}

	#[test]
	fn covering_fit_claims_every_passage_on_corner_overhang_strip() {
		let confines = strip_with_doors(&[10.0, 20.0, 30.0], 40.0);
		let min = Vec3::ZERO;
		let max = Vec3::new(40.0, 3.5, 5.0);
		let passages = collect_passages_along(&confines.openings, true, min, 40.0);
		let bays = partition_bays_for_passages(&passages, 40.0, 3.5);
		let stalls =
			fit_stalls_covering_strip(&confines, true, min, max, &bays, NoiseParams::default())
				.unwrap();
		assert_eq!(stalls.len(), bays.len());
		let mut seen = std::collections::HashSet::new();
		for bay in &bays {
			assert!(!bay.passage_ids.is_empty());
			for id in &bay.passage_ids {
				assert!(seen.insert(id.clone()), "passage {} claimed twice", id.as_str());
			}
		}
		assert_eq!(seen.len(), passages.len());
	}
}
