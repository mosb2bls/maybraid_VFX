//! Shared greedy furniture pack loop for usage-area rooms.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::Vec3;
use procedural_common::{aabb2_area, aabb3_to_plan, NoiseConfig, NoiseParams, PlanAxes};

use crate::fit::{Confines, FitError};
use crate::usage_areas::clearance::{approach_zone, PassageClearance, PASSAGE_WALL_LIP};

use super::predicates::{all_pass, PredicateCtx};
use super::{
	pick_kind, try_free_extent, try_wall_long, FreeExtentKnobs, KindSpec, OccupiedBudget,
	ProposeKnobs, SoftGoalRole, WallLongKnobs,
};

pub const WALL_EPS: f32 = 0.08;
const PROPOSE_ATTEMPTS: u32 = 22;

/// Options for [`init_host_with`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct InitHostOpts {
	/// Add a shallow full-wall lip on every passage wall (open kitchens / living).
	pub passage_wall_lip: bool,
}

/// Host plan + passage keep-outs for a furniture pack.
pub struct PackHost {
	pub host3: Aabb3d,
	pub host: Aabb2d,
	pub clearances: Vec<Aabb2d>,
	/// Passage bands only (before furniture commits) — for approach-padded excludes.
	pub passage_bands: Vec<Aabb2d>,
	pub room_area: f32,
}

impl PackHost {
	/// Push a placed solid's plan footprint into passage keep-outs.
	pub fn commit_footprint(&mut self, solid: &Aabb3d) {
		self.clearances.push(aabb3_to_plan(solid, PlanAxes::XZ));
	}

	/// Clearances plus padded door approaches — use for fillers that tend to
	/// sit beside a passage face just outside the strict keep-out band.
	pub fn clearances_with_approach(&self) -> Vec<Aabb2d> {
		let mut out = self.clearances.clone();
		for band in &self.passage_bands {
			out.push(approach_zone(*band));
		}
		out
	}
}

/// Knobs shared by furniture pack loops.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackKnobs {
	pub spaciousness: f32,
	pub furniture_occupancy: f32,
	pub structure_occupancy: f32,
	pub max_steps: u32,
}

impl PackKnobs {
	pub fn budget(&self, room_area: f32) -> OccupiedBudget {
		OccupiedBudget::new(room_area, self.furniture_occupancy, self.structure_occupancy)
	}
}

/// Initialize host geometry and passage clearance bands (passage required).
pub fn init_host(confines: &Confines) -> Result<PackHost, FitError> {
	init_host_with(confines, InitHostOpts::default())
}

/// Like [`init_host`], with open-room clearance options.
pub fn init_host_with(confines: &Confines, opts: InitHostOpts) -> Result<PackHost, FitError> {
	let host3 = confines.bounds;
	let host = aabb3_to_plan(&host3, PlanAxes::XZ);
	// Residential packs wall-snap faces so flush counters/sofas meet keep-outs
	// that otherwise start inset at the inner door volume.
	let passage_faces = PassageClearance::collect_faces_wall_snapped(confines, host);
	if passage_faces.is_empty() {
		return Err(FitError::TooSmall { reason: "passage" });
	}
	let mut passage_bands = PassageClearance::bands_std(host, &passage_faces);
	if opts.passage_wall_lip {
		passage_bands.extend(PassageClearance::bands_wall_lip(
			host,
			&passage_faces,
			PASSAGE_WALL_LIP,
		));
	}
	let clearances = passage_bands.clone();
	let room_area = aabb2_area(host).max(1e-4);
	Ok(PackHost { host3, host, clearances, passage_bands, room_area })
}

/// Plan-area of an AABB footprint on XZ.
pub fn xz_area(aabb: &Aabb3d) -> f32 {
	let e = aabb.max - aabb.min;
	e.x.max(0.0) * e.z.max(0.0)
}

/// True when any placed kind credits the enclosure soft-goal via its spec role.
pub fn soft_goal_from_placed<Kind: Copy + PartialEq>(
	catalog: &[KindSpec<Kind>],
	placed: &[(Kind, Aabb3d)],
) -> bool {
	placed.iter().any(|(kind, _)| {
		catalog.iter().find(|s| s.id == *kind).is_some_and(|s| {
			matches!(
				s.soft_goal,
				SoftGoalRole::ClosetLike | SoftGoalRole::Appointed | SoftGoalRole::Ensuite
			)
		})
	})
}

/// Propose a candidate AABB from a kind spec's propose knobs.
pub fn propose_from_spec<Kind>(
	spec: &KindSpec<Kind>,
	spaciousness: f32,
	host3: &Aabb3d,
	host: Aabb2d,
	clearances: &[Aabb2d],
	cfg: &NoiseConfig,
	salt: u32,
	center: Vec3,
) -> Option<Aabb3d> {
	let span_x = (host.max.x - host.min.x).max(0.0);
	let span_z = (host.max.y - host.min.y).max(0.0);
	let long_span = span_x.max(span_z);
	let short_span = span_x.min(span_z);

	match spec.propose {
		ProposeKnobs::FreeExtent { min_x, max_x, min_z, max_z, height, prefer_wall } => {
			let sx = cfg.sample_range_f32_4d(
				min_x * spaciousness,
				max_x * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 1.0,
			);
			let sz = cfg.sample_range_f32_4d(
				min_z * spaciousness,
				max_z * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 2.0,
			);
			let h = height * spaciousness.min(1.2);
			try_free_extent(
				host3,
				host,
				clearances,
				cfg,
				salt,
				FreeExtentKnobs {
					extent: Vec3::new(sx, h, sz),
					prefer_wall,
					wall_eps: WALL_EPS,
					attempts: PROPOSE_ATTEMPTS,
				},
			)
		}
		ProposeKnobs::FreeExtentFrac {
			long_frac_min,
			long_frac_max,
			short_frac_min,
			short_frac_max,
			height,
			prefer_wall,
			align_long_to_host,
		} => {
			let along_lo = long_span * long_frac_min * spaciousness;
			let along_hi = (long_span * long_frac_max * spaciousness).min(long_span - 0.35);
			let depth_lo = short_span * short_frac_min * spaciousness;
			let depth_hi = (short_span * short_frac_max * spaciousness).min(short_span - 0.35);
			if along_hi < along_lo + 1e-3 || depth_hi < depth_lo + 1e-3 {
				return None;
			}
			let along = cfg.sample_range_f32_4d(
				along_lo,
				along_hi,
				center.x,
				center.y,
				center.z,
				salt as f32 + 1.0,
			);
			let depth = cfg.sample_range_f32_4d(
				depth_lo,
				depth_hi,
				center.x,
				center.y,
				center.z,
				salt as f32 + 2.0,
			);
			let h = height * spaciousness.min(1.2);
			let primary = if !align_long_to_host || span_x >= span_z {
				Vec3::new(along, h, depth)
			} else {
				Vec3::new(depth, h, along)
			};
			let secondary = Vec3::new(primary.z, primary.y, primary.x);
			for (i, extent) in [primary, secondary].into_iter().enumerate() {
				if let Some(aabb) = try_free_extent(
					host3,
					host,
					clearances,
					cfg,
					salt + i as u32,
					FreeExtentKnobs {
						extent,
						prefer_wall,
						wall_eps: WALL_EPS,
						attempts: PROPOSE_ATTEMPTS,
					},
				) {
					return Some(aabb);
				}
			}
			None
		}
		ProposeKnobs::WallLong { along_min, along_max, depth_min, depth_max, height } => {
			let along = cfg.sample_range_f32_4d(
				along_min * spaciousness,
				along_max * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 3.0,
			);
			let depth = cfg.sample_range_f32_4d(
				depth_min * spaciousness,
				depth_max * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 4.0,
			);
			let h = height * spaciousness.min(1.2);
			try_wall_long(
				host3,
				host,
				clearances,
				cfg,
				salt,
				WallLongKnobs {
					extent: Vec3::new(along, h, depth),
					wall_eps: WALL_EPS,
					attempts: PROPOSE_ATTEMPTS,
				},
			)
		}
		ProposeKnobs::WallLongFrac {
			along_frac_min,
			along_frac_max,
			depth_min,
			depth_max,
			height,
		} => {
			let max_along = long_span;
			let along = cfg.sample_range_f32_4d(
				along_frac_min * max_along * spaciousness,
				(along_frac_max * max_along * spaciousness).min(max_along - 0.15),
				center.x,
				center.y,
				center.z,
				salt as f32 + 3.0,
			);
			let depth = cfg.sample_range_f32_4d(
				depth_min * spaciousness,
				depth_max * spaciousness,
				center.x,
				center.y,
				center.z,
				salt as f32 + 4.0,
			);
			let h = height * spaciousness.min(1.2);
			try_wall_long(
				host3,
				host,
				clearances,
				cfg,
				salt,
				WallLongKnobs {
					extent: Vec3::new(along, h, depth),
					wall_eps: WALL_EPS,
					attempts: PROPOSE_ATTEMPTS,
				},
			)
		}
		ProposeKnobs::EnclosedRoom => None,
	}
}

fn count_kind<Kind: Copy + PartialEq>(placed: &[(Kind, Aabb3d)], kind: Kind) -> usize {
	placed.iter().filter(|(k, _)| *k == kind).count()
}

/// Greedy kind loop: pick → propose → predicate → budget → commit.
pub fn pack_kinds<Kind: Copy + PartialEq>(
	catalog: &[KindSpec<Kind>],
	knobs: PackKnobs,
	host: &mut PackHost,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<Vec<(Kind, Aabb3d)>, FitError> {
	let cfg = NoiseConfig::new(noise);
	let center = confines.center();
	let mut budget = knobs.budget(host.room_area);
	let mut placed = Vec::new();

	for step in 0..knobs.max_steps {
		if budget.furniture_full() {
			break;
		}
		let soft_goal_met = soft_goal_from_placed(catalog, &placed);
		let Some(kind) = pick_kind(catalog, &cfg, step, soft_goal_met, |k| count_kind(&placed, k))
		else {
			break;
		};
		let Some(spec) = catalog.iter().find(|s| s.id == kind) else {
			continue;
		};
		let Some(candidate) = propose_from_spec(
			spec,
			knobs.spaciousness,
			&host.host3,
			host.host,
			&host.clearances,
			&cfg,
			step + 100,
			center,
		) else {
			continue;
		};
		let plan = aabb3_to_plan(&candidate, PlanAxes::XZ);
		let ctx = PredicateCtx {
			host: host.host,
			candidate: plan,
			clearances: &host.clearances,
			door_clear: None,
			wall_eps: WALL_EPS,
		};
		if !all_pass(spec.predicates, ctx) {
			continue;
		}
		let add = xz_area(&candidate);
		if !budget.accepts(add, spec.structure_budget) {
			continue;
		}
		budget.commit(add);
		host.commit_footprint(&candidate);
		placed.push((kind, candidate));
	}

	Ok(placed)
}
