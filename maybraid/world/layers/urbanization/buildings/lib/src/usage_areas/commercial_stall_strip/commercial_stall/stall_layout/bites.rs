//! Bites counter / seating / kitchen packing on passage plan faces.
//!
//! Counters bind to long faces of customer passage openings via
//! [`PlanOpeningFace`]. Sit-down seating seeds on those faces and grows with a
//! kitchen area reserve; kitchen is max-empty then `grow_into`.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec2;
use procedural_common::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, inflate_aabb2, max_empty_rect2, plan_to_aabb3,
	Aabb2dPack, OptionalFaceBand, PlanAxes, PlanOpeningFace,
};

use crate::fit::{Confines, FitError};
use crate::openings::OpeningLabel;

/// Passages must be at least this long (along-wall) to host a BitesCounter.
pub const BITES_LONG_PASSAGE_MIN: f32 = 2.0;
/// Counter along-length floor; the rest of the passage (≥1m) stays clear.
pub const BITES_COUNTER_ALONG_MIN: f32 = 1.0;
/// Clear passage length left beside each counter.
pub const BITES_PASSAGE_REMAIN_MIN: f32 = 1.0;
/// Kitchen stays at least this far (XZ) from every counter.
pub const BITES_KITCHEN_COUNTER_CLEARANCE: f32 = 1.0;
/// Kitchen / seating plan minimum (width and depth).
pub const BITES_REGION_MIN_PLAN: f32 = 1.0;
/// Minimum shared border between seating and a passage long face.
pub const BITES_SEATING_FACE_CONTACT: f32 = 1.0;

/// Default place-rate when sampling counter presence on a long passage.
pub const BITES_COUNTER_PLACE_RATE: f32 = 0.60;

/// Per-passage counter choice ([`OptionalFaceBand`] alias for domain naming).
pub type BitesCounterChoice = OptionalFaceBand;

/// A long Passage with its opening face into the stall (bites source of truth).
#[derive(Debug, Clone, PartialEq)]
pub struct EligibleBitesPassage {
	pub bounds: Aabb3d,
	pub face: PlanOpeningFace,
	pub along_len: f32,
}

impl EligibleBitesPassage {
	/// Collect passages ≥ [`BITES_LONG_PASSAGE_MIN`] with a resolvable opening face.
	///
	/// Uses [`PlanOpeningFace::from_passage`] only (along length = face along span).
	///
	/// `confines.openings` should already be the bay-owned set (strip partition
	/// filters passages per cell before interior fit).
	pub fn collect(confines: &Confines) -> Vec<Self> {
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let mut out = Vec::new();
		for (_id, opening) in confines.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let passage_plan = aabb3_to_plan(&opening.bounds, PlanAxes::XZ);
			let Some(face) = PlanOpeningFace::from_passage(host, passage_plan) else {
				continue;
			};
			let along_len = face.along_len();
			if along_len + 1e-3 < BITES_LONG_PASSAGE_MIN {
				continue;
			}
			if along_len + 1e-3 < BITES_COUNTER_ALONG_MIN {
				continue;
			}
			out.push(Self { bounds: opening.bounds, face, along_len });
		}
		out
	}
}

/// Eligible passage snapshotted with its counter choice (no parallel-array drift).
#[derive(Debug, Clone, PartialEq)]
pub struct BitesPassageSpec {
	pub passage: EligibleBitesPassage,
	pub counter: BitesCounterChoice,
}

impl BitesPassageSpec {
	pub fn pack_counter(&self, host_bounds: &Aabb3d) -> Option<Aabb3d> {
		let host = aabb3_to_plan(host_bounds, PlanAxes::XZ);
		self.counter
			.resolve(host, self.passage.face)
			.map(|c| plan_to_aabb3(host_bounds, c, PlanAxes::XZ))
	}
}

/// Counters packed from a passage snapshot.
#[derive(Debug, Clone)]
pub struct PackedBitesCounters {
	pub counters: Vec<Aabb3d>,
	pub faces: Vec<PlanOpeningFace>,
	pub specs: Vec<BitesPassageSpec>,
}

impl PackedBitesCounters {
	pub fn from_specs(host_bounds: &Aabb3d, specs: &[BitesPassageSpec]) -> Result<Self, FitError> {
		if specs.is_empty() {
			return Err(FitError::TooSmall { reason: "bites counter passage" });
		}
		let mut counters = Vec::new();
		let mut faces = Vec::with_capacity(specs.len());
		for spec in specs {
			faces.push(spec.passage.face);
			if let Some(c) = spec.pack_counter(host_bounds) {
				counters.push(c);
			}
		}
		if counters.is_empty() {
			return Err(FitError::TooSmall { reason: "bites counter passage" });
		}
		Ok(Self { counters, faces, specs: specs.to_vec() })
	}
}

/// Kitchen packing: max empty with counter clearance (+ optional hard excludes).
pub struct BitesKitchen;

impl BitesKitchen {
	/// Single path: inflate counters by clearance, union `extra_excludes`, max-empty.
	pub fn pack(
		bounds: &Aabb3d,
		counters: &[Aabb3d],
		extra_excludes: &[Aabb3d],
		min_plan: f32,
	) -> Option<Aabb3d> {
		let host = aabb3_to_plan(bounds, PlanAxes::XZ);
		let mut cuts: Vec<Aabb2d> = counters
			.iter()
			.map(|c| inflate_aabb2(aabb3_to_plan(c, PlanAxes::XZ), BITES_KITCHEN_COUNTER_CLEARANCE))
			.collect();
		cuts.extend(extra_excludes.iter().map(|e| aabb3_to_plan(e, PlanAxes::XZ)));
		let kitchen2 = max_empty_rect2(host, &cuts)?;
		let kitchen2 = clamp_min_size2(kitchen2, Vec2::splat(min_plan))?;
		Some(plan_to_aabb3(bounds, kitchen2, PlanAxes::XZ))
	}
}

/// Sit-down seating + kitchen packer (staged).
pub struct BitesSitdownRegions {
	pub seating_area_target: f32,
	pub seating_contact: f32,
	pub seating_seed_depth: f32,
	pub seating_along_t: f32,
	pub kitchen_area_reserve: f32,
	pub min_plan: f32,
}

impl BitesSitdownRegions {
	pub fn pack(
		&self,
		bounds: &Aabb3d,
		counters: &[Aabb3d],
		faces: &[PlanOpeningFace],
	) -> Option<(Aabb3d, Aabb3d)> {
		let (seed, face) = self.seed_seating(bounds, counters, faces)?;
		let seating = self.grow_seating(bounds, counters, seed, face)?;
		let kitchen = self.pack_kitchen(bounds, counters, &seating)?;
		Some((seating, kitchen))
	}

	fn seed_seating(
		&self,
		bounds: &Aabb3d,
		counters: &[Aabb3d],
		faces: &[PlanOpeningFace],
	) -> Option<(Aabb2d, PlanOpeningFace)> {
		let host = aabb3_to_plan(bounds, PlanAxes::XZ);
		let counter_plans: Vec<_> =
			counters.iter().map(|c| aabb3_to_plan(c, PlanAxes::XZ)).collect();
		let contact = self.seating_contact.max(BITES_SEATING_FACE_CONTACT);
		let depth = self.seating_seed_depth.max(self.min_plan);

		let mut order: Vec<usize> = (0..faces.len()).collect();
		order.sort_by(|a, b| {
			faces[*b]
				.along_len()
				.partial_cmp(&faces[*a].along_len())
				.unwrap_or(std::cmp::Ordering::Equal)
		});
		for &i in &order {
			if let Some(seed) =
				faces[i].seed_from_free(host, &counter_plans, contact, depth, self.seating_along_t)
			{
				return Some((seed, faces[i]));
			}
			// Fall back across the free segment if the noisy t misses.
			if let Some((seg0, seg1)) = faces[i].longest_free_segment(&counter_plans, contact) {
				let span = seg1 - seg0;
				if span + 1e-3 >= contact {
					if let Some(seed) =
						faces[i].seed_from_free(host, &counter_plans, contact, depth, 0.5)
					{
						return Some((seed, faces[i]));
					}
				}
			}
		}
		None
	}

	fn grow_seating(
		&self,
		bounds: &Aabb3d,
		counters: &[Aabb3d],
		seed: Aabb2d,
		face: PlanOpeningFace,
	) -> Option<Aabb3d> {
		let host = aabb3_to_plan(bounds, PlanAxes::XZ);
		let counter_plans: Vec<_> =
			counters.iter().map(|c| aabb3_to_plan(c, PlanAxes::XZ)).collect();
		let contact = self.seating_contact.max(BITES_SEATING_FACE_CONTACT);
		let mut hard = counter_plans;
		hard.push(face.outward_block(host));

		let counter_area: f32 =
			counters.iter().map(|c| aabb2_area(aabb3_to_plan(c, PlanAxes::XZ))).sum();
		let usable = (aabb2_area(host) - counter_area).max(0.0);
		let kitchen_reserve =
			self.kitchen_area_reserve.max(self.min_plan * self.min_plan).min(usable * 0.35);
		let target = self
			.seating_area_target
			.max(self.min_plan * self.min_plan)
			.min((usable - kitchen_reserve).max(self.min_plan * self.min_plan));

		let seating2 = seed.grow_toward_area(host, &hard, target);
		let seating2 = clamp_min_size2(seating2, Vec2::splat(self.min_plan))?;
		if face.shared_border_len(seating2) + 1e-3 < contact {
			return None;
		}
		Some(plan_to_aabb3(bounds, seating2, PlanAxes::XZ))
	}

	fn pack_kitchen(
		&self,
		bounds: &Aabb3d,
		counters: &[Aabb3d],
		seating: &Aabb3d,
	) -> Option<Aabb3d> {
		let kitchen_seed = BitesKitchen::pack(bounds, counters, &[*seating], self.min_plan)?;
		let host = aabb3_to_plan(bounds, PlanAxes::XZ);
		let seating2 = aabb3_to_plan(seating, PlanAxes::XZ);
		let kitchen2 = aabb3_to_plan(&kitchen_seed, PlanAxes::XZ);
		let mut hard: Vec<_> = counters
			.iter()
			.map(|c| inflate_aabb2(aabb3_to_plan(c, PlanAxes::XZ), BITES_KITCHEN_COUNTER_CLEARANCE))
			.collect();
		hard.push(seating2);
		let kitchen2 = kitchen2.grow_into(host, &hard);
		let kitchen2 = clamp_min_size2(kitchen2, Vec2::splat(self.min_plan))?;
		Some(plan_to_aabb3(bounds, kitchen2, PlanAxes::XZ))
	}
}
