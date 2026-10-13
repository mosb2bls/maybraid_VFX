//! Noise-sampled knobs for a Les Halles floor plan.

use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{Confines, FitError};
use crate::openings::{
	fit_bays_on_run, fit_windows_on_run, generate_stall_doors, generate_windows, BaySpec, PlacedBay,
};

/// Where vertical shafts are allocated in the gallery ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LesHallesShaftPlacement {
	/// One shaft in each of the four corners of the gallery ring.
	Corners,
	/// One shaft in the middle of each of the four sides.
	MidSides,
}

/// Typology alias for shared [`BaySpec`] (stall doors / windows).
pub type LesHallesStallDoor = BaySpec;
/// Typology alias for shared [`PlacedBay`].
pub type LesHallesPlacedDoor = PlacedBay;

/// Resolved Les Halles plan knobs.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesParameterized {
	/// Outer commercial gallery / stall band depth (meters).
	pub gallery_width: f32,
	/// Inner balcony / walking band depth (meters).
	pub balcony_width: f32,
	/// Target courtyard share of the short footprint axis (`~0.5` → ¼ rim / ½ gap / ¼ rim).
	pub courtyard_fraction: f32,
	pub shaft_placement: LesHallesShaftPlacement,
	/// Along-wall side length for mid-side shafts (scales with footprint).
	pub mid_shaft_side: f32,
	/// How densely to pack outer-facade apertures (`0…1`).
	pub opening_density: f32,
	/// Stall door sizes to pack along each inner-wall straight section (catalog order).
	pub doors: Vec<LesHallesStallDoor>,
	/// Exterior aperture sizes to pack along outer free runs (catalog order).
	pub windows: Vec<LesHallesStallDoor>,
}

/// Minimum gallery / stall band depth.
pub const MIN_GALLERY_WIDTH: f32 = 5.0;
/// Maximum gallery / stall band depth sampled from noise.
pub const MAX_GALLERY_WIDTH: f32 = 25.0;
/// Minimum balcony band depth.
pub const MIN_BALCONY_WIDTH: f32 = 3.0;
/// Maximum balcony band depth sampled from noise.
pub const MAX_BALCONY_WIDTH: f32 = 8.0;
/// Minimum clear courtyard on each plan axis (hard floor).
pub const MIN_COURTYARD: f32 = 2.0;
/// Minimum storey height.
pub const MIN_STOREY_HEIGHT: f32 = 2.5;
/// Minimum along-wall side length for mid-side shafts.
pub const MIN_MID_SHAFT_SIDE: f32 = 2.4;
/// Maximum along-wall side length for mid-side shafts.
pub const MAX_MID_SHAFT_SIDE: f32 = 8.0;
/// Legacy alias for [`MIN_MID_SHAFT_SIDE`].
pub const SHAFT_SIDE: f32 = MIN_MID_SHAFT_SIDE;

/// Default plan split targets ~½ courtyard (¼ rim + ½ gap + ¼ rim).
pub const MIN_COURTYARD_FRACTION: f32 = 0.40;
pub const MAX_COURTYARD_FRACTION: f32 = 0.60;
/// Stall share of the rim budget (gallery / (gallery + balcony)).
pub const MIN_STALL_RING_SHARE: f32 = 0.55;
pub const MAX_STALL_RING_SHARE: f32 = 0.75;

/// Minimum gallery depth for [`LesHallesParameterized::sample_livable`].
pub const MIN_LIVABLE_GALLERY_WIDTH: f32 = 8.0;
/// Soft max gallery depth preferred by livable sampling (still clamped by footprint).
pub const MAX_LIVABLE_GALLERY_WIDTH: f32 = 16.0;
/// Stall-ring share range for livable gallery bias (deeper apartment band).
pub const MIN_LIVABLE_STALL_RING_SHARE: f32 = 0.70;
pub const MAX_LIVABLE_STALL_RING_SHARE: f32 = 0.85;

/// Minimum gallery depth for [`LesHallesParameterized::sample_monotower`].
pub const MIN_MONOTOWER_GALLERY_WIDTH: f32 = 7.0;
/// Soft max gallery depth for mixed-use monotower shells.
pub const MAX_MONOTOWER_GALLERY_WIDTH: f32 = 14.0;
/// Stall-ring share for monotower — between commercial and livable.
pub const MIN_MONOTOWER_STALL_RING_SHARE: f32 = 0.62;
pub const MAX_MONOTOWER_STALL_RING_SHARE: f32 = 0.78;
/// Hard courtyard floor for monotower (expects larger footprints than commercial).
pub const MIN_MONOTOWER_COURTYARD: f32 = 8.0;
/// Gallery depth so a corner keep can sit on the curtain wall, not hang off it.
pub const MIN_CURTAIN_GALLERY_WIDTH: f32 = 16.0;
/// Soft max gallery depth for ring-fort curtain walls.
pub const MAX_CURTAIN_GALLERY_WIDTH: f32 = 40.0;
/// Stall-ring share for curtain walls — gallery takes most of the rim.
pub const MIN_CURTAIN_STALL_RING_SHARE: f32 = 0.78;
pub const MAX_CURTAIN_STALL_RING_SHARE: f32 = 0.90;
/// Courtyard still open, but the rim is the load-bearing mass.
pub const MIN_CURTAIN_COURTYARD_FRACTION: f32 = 0.22;
pub const MAX_CURTAIN_COURTYARD_FRACTION: f32 = 0.38;
/// Clear courtyard floor for a curtain-wall ring.
pub const MIN_CURTAIN_COURTYARD: f32 = 24.0;
/// Storey height range for Les Halles monotowers (meters).
pub const MIN_MONOTOWER_STOREY_HEIGHT: f32 = 3.0;
pub const MAX_MONOTOWER_STOREY_HEIGHT: f32 = 5.0;
/// Mixed-use stack: arcade plus upper commercial / livable floors.
pub const MIN_MONOTOWER_STOREYS: usize = 2;
pub const MAX_MONOTOWER_STOREYS: usize = 7;

/// Salt lanes for spatial sampling at the confines center.
const SALT_COURTYARD: f32 = 1.0;
const SALT_STALL_SHARE: f32 = 2.0;
const SALT_SHAFT: f32 = 3.0;
const SALT_MID_SHAFT: f32 = 3.5;
const SALT_OPENINGS: f32 = 4.0;
const SALT_STOREY_HEIGHT: f32 = 5.0;
const SALT_COMMERCIAL_COUNT: f32 = 6.0;
const SALT_SHAFT_COUNT: f32 = 7.0;
const SALT_SHAFT_PICK: f32 = 7.5;
const SALT_STOREY_COUNT: f32 = 8.0;

impl LesHallesParameterized {
	/// Sample knobs at the confines center. Rejects footprints that cannot host
	/// minimum gallery + balcony + courtyard on either axis.
	///
	/// Depths are driven by a courtyard-fraction target (default ~½ the short
	/// axis), then the remaining rim is split into stall vs balcony within
	/// absolute min/max clamps. Mins win over the ratio when the footprint is tight.
	///
	/// Stall-door / window catalogs come from [`crate::openings::generate_stall_doors`]
	/// / [`crate::openings::generate_windows`].
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let extent_min = extent_x.min(extent_z);

		let courtyard_fraction = cfg.sample_range_f32_4d(
			MIN_COURTYARD_FRACTION,
			MAX_COURTYARD_FRACTION,
			c.x,
			c.y,
			c.z,
			SALT_COURTYARD,
		);

		// Rim budget from the ratio, then clamp so mins/maxes and MIN_COURTYARD hold.
		let ideal_ring = extent_min * (1.0 - courtyard_fraction) * 0.5;
		let hard_max_ring = ((extent_min - MIN_COURTYARD) * 0.5).max(min_ring);
		let abs_max_ring = MAX_GALLERY_WIDTH + MAX_BALCONY_WIDTH;
		let ring_budget = ideal_ring.clamp(min_ring, hard_max_ring.min(abs_max_ring));

		let stall_share = cfg.sample_range_f32_4d(
			MIN_STALL_RING_SHARE,
			MAX_STALL_RING_SHARE,
			c.x,
			c.y,
			c.z,
			SALT_STALL_SHARE,
		);
		let (gallery_width, balcony_width) = split_ring_budget(ring_budget, stall_share)?;

		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_COURTYARD || extent_z < 2.0 * ring + MIN_COURTYARD {
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		// Mid-side shafts grow with the short footprint axis (room for stairs).
		let mid_hi = (extent_min * 0.15).clamp(3.5, MAX_MID_SHAFT_SIDE);
		let mid_lo = MIN_MID_SHAFT_SIDE.min(mid_hi);
		let mid_shaft_side = cfg.sample_range_f32_4d(mid_lo, mid_hi, c.x, c.y, c.z, SALT_MID_SHAFT);

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let doors = generate_stall_doors(&cfg, c);
		let windows = generate_windows(&cfg, c);

		Ok(Self {
			gallery_width,
			balcony_width,
			courtyard_fraction,
			shaft_placement,
			mid_shaft_side,
			opening_density,
			doors,
			windows,
		})
	}

	/// Like [`Self::sample`], but biases the gallery toward apartment-friendly
	/// depths (~8–16 m) via a higher stall-ring share.
	///
	/// Used by [`super::LesHallesLivableFullStorey`]; commercial
	/// [`Self::sample`] is unchanged.
	pub fn sample_livable(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_LIVABLE_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();
		let extent_min = extent_x.min(extent_z);

		let courtyard_fraction = cfg.sample_range_f32_4d(
			MIN_COURTYARD_FRACTION,
			MAX_COURTYARD_FRACTION,
			c.x,
			c.y,
			c.z,
			SALT_COURTYARD,
		);

		let ideal_ring = extent_min * (1.0 - courtyard_fraction) * 0.5;
		let hard_max_ring = ((extent_min - MIN_COURTYARD) * 0.5).max(min_ring);
		let abs_max_ring = MAX_LIVABLE_GALLERY_WIDTH + MAX_BALCONY_WIDTH;
		let ring_budget = ideal_ring.clamp(min_ring, hard_max_ring.min(abs_max_ring));

		let stall_share = cfg.sample_range_f32_4d(
			MIN_LIVABLE_STALL_RING_SHARE,
			MAX_LIVABLE_STALL_RING_SHARE,
			c.x,
			c.y,
			c.z,
			SALT_STALL_SHARE,
		);
		let (gallery_width, balcony_width) = split_ring_budget_livable(ring_budget, stall_share)?;

		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_COURTYARD || extent_z < 2.0 * ring + MIN_COURTYARD {
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		let mid_hi = (extent_min * 0.15).clamp(3.5, MAX_MID_SHAFT_SIDE);
		let mid_lo = MIN_MID_SHAFT_SIDE.min(mid_hi);
		let mid_shaft_side = cfg.sample_range_f32_4d(mid_lo, mid_hi, c.x, c.y, c.z, SALT_MID_SHAFT);

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let doors = generate_stall_doors(&cfg, c);
		let windows = generate_windows(&cfg, c);

		Ok(Self {
			gallery_width,
			balcony_width,
			courtyard_fraction,
			shaft_placement,
			mid_shaft_side,
			opening_density,
			doors,
			windows,
		})
	}

	/// Shared shell knobs for mixed-use monotowers: gallery depth between
	/// commercial and livable, with a larger courtyard floor so apartments and
	/// stalls both fit on big footprints.
	///
	/// Spatial noise uses the plan-center with \(y = 0\) so slicing the tower
	/// AABB into storeys does not change the shared shell knobs.
	pub fn sample_monotower(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_MONOTOWER_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_MONOTOWER_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_MONOTOWER_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		let extent_min = extent_x.min(extent_z);

		let courtyard_fraction = cfg.sample_range_f32_4d(
			MIN_COURTYARD_FRACTION,
			MAX_COURTYARD_FRACTION,
			c.x,
			c.y,
			c.z,
			SALT_COURTYARD,
		);

		let ideal_ring = extent_min * (1.0 - courtyard_fraction) * 0.5;
		let hard_max_ring = ((extent_min - MIN_MONOTOWER_COURTYARD) * 0.5).max(min_ring);
		let abs_max_ring = MAX_MONOTOWER_GALLERY_WIDTH + MAX_BALCONY_WIDTH;
		let ring_budget = ideal_ring.clamp(min_ring, hard_max_ring.min(abs_max_ring));

		let stall_share = cfg.sample_range_f32_4d(
			MIN_MONOTOWER_STALL_RING_SHARE,
			MAX_MONOTOWER_STALL_RING_SHARE,
			c.x,
			c.y,
			c.z,
			SALT_STALL_SHARE,
		);
		let (gallery_width, balcony_width) = split_ring_budget_monotower(ring_budget, stall_share)?;

		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_MONOTOWER_COURTYARD
			|| extent_z < 2.0 * ring + MIN_MONOTOWER_COURTYARD
		{
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		let mid_hi = (extent_min * 0.15).clamp(3.5, MAX_MID_SHAFT_SIDE);
		let mid_lo = MIN_MID_SHAFT_SIDE.min(mid_hi);
		let mid_shaft_side = cfg.sample_range_f32_4d(mid_lo, mid_hi, c.x, c.y, c.z, SALT_MID_SHAFT);

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let doors = generate_stall_doors(&cfg, c);
		let windows = generate_windows(&cfg, c);

		Ok(Self {
			gallery_width,
			balcony_width,
			courtyard_fraction,
			shaft_placement,
			mid_shaft_side,
			opening_density,
			doors,
			windows,
		})
	}

	/// Deep gallery ring for a courtyard curtain wall that can carry corner keeps.
	pub fn sample_curtain(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let (extent_x, extent_z, height) = footprint_extents(confines)?;
		let min_ring = MIN_CURTAIN_GALLERY_WIDTH + MIN_BALCONY_WIDTH;
		let min_outer = 2.0 * min_ring + MIN_CURTAIN_COURTYARD;
		if extent_x < min_outer || extent_z < min_outer {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_MONOTOWER_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		let extent_min = extent_x.min(extent_z);

		let courtyard_fraction = cfg.sample_range_f32_4d(
			MIN_CURTAIN_COURTYARD_FRACTION,
			MAX_CURTAIN_COURTYARD_FRACTION,
			c.x,
			c.y,
			c.z,
			SALT_COURTYARD,
		);

		let ideal_ring = extent_min * (1.0 - courtyard_fraction) * 0.5;
		let hard_max_ring = ((extent_min - MIN_CURTAIN_COURTYARD) * 0.5).max(min_ring);
		let abs_max_ring = MAX_CURTAIN_GALLERY_WIDTH + MAX_BALCONY_WIDTH;
		let ring_budget = ideal_ring.clamp(min_ring, hard_max_ring.min(abs_max_ring));

		let stall_share = cfg.sample_range_f32_4d(
			MIN_CURTAIN_STALL_RING_SHARE,
			MAX_CURTAIN_STALL_RING_SHARE,
			c.x,
			c.y,
			c.z,
			SALT_STALL_SHARE,
		);
		let (gallery_width, balcony_width) = split_ring_budget_curtain(ring_budget, stall_share)?;

		let ring = gallery_width + balcony_width;
		if extent_x < 2.0 * ring + MIN_CURTAIN_COURTYARD
			|| extent_z < 2.0 * ring + MIN_CURTAIN_COURTYARD
		{
			return Err(FitError::TooSmall { reason: "footprint" });
		}

		let shaft_i = cfg.sample_range_usize_4d(0, 2, c.x, c.y, c.z, SALT_SHAFT);
		let shaft_placement = if shaft_i == 0 {
			LesHallesShaftPlacement::Corners
		} else {
			LesHallesShaftPlacement::MidSides
		};

		let mid_hi = (extent_min * 0.15).clamp(3.5, MAX_MID_SHAFT_SIDE);
		let mid_lo = MIN_MID_SHAFT_SIDE.min(mid_hi);
		let mid_shaft_side = cfg.sample_range_f32_4d(mid_lo, mid_hi, c.x, c.y, c.z, SALT_MID_SHAFT);

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let doors = generate_stall_doors(&cfg, c);
		let windows = generate_windows(&cfg, c);

		Ok(Self {
			gallery_width,
			balcony_width,
			courtyard_fraction,
			shaft_placement,
			mid_shaft_side,
			opening_density,
			doors,
			windows,
		})
	}

	/// Sample a storey height in `[MIN_MONOTOWER_STOREY_HEIGHT, MAX_MONOTOWER_STOREY_HEIGHT]`.
	pub fn sample_monotower_storey_height(confines: &Confines, noise: NoiseParams) -> f32 {
		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		cfg.sample_range_f32_4d(
			MIN_MONOTOWER_STOREY_HEIGHT,
			MAX_MONOTOWER_STOREY_HEIGHT,
			c.x,
			c.y,
			c.z,
			SALT_STOREY_HEIGHT,
		)
	}

	/// How many storeys to author in `[MIN_MONOTOWER_STOREYS, MAX_MONOTOWER_STOREYS]`,
	/// clipped to what `confines` height can host at [`MIN_MONOTOWER_STOREY_HEIGHT`].
	pub fn sample_monotower_storey_count(confines: &Confines, noise: NoiseParams) -> usize {
		let total_h = footprint_extents(confines).map(|(_, _, h)| h).unwrap_or(0.0);
		let max_n = ((total_h / MIN_MONOTOWER_STOREY_HEIGHT).floor() as usize)
			.clamp(1, MAX_MONOTOWER_STOREYS);
		let min_n = MIN_MONOTOWER_STOREYS.min(max_n).max(1);
		if min_n == max_n {
			return min_n;
		}
		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		min_n + cfg.sample_range_usize_4d(0, max_n - min_n + 1, c.x, c.y, c.z, SALT_STOREY_COUNT)
	}

	/// Number of commercial storeys from the ground up (`1…n_storeys`, or `1…n-1`
	/// when `n_storeys ≥ 2` so at least one floor stays residential).
	pub fn sample_monotower_commercial_count(
		confines: &Confines,
		noise: NoiseParams,
		n_storeys: usize,
	) -> usize {
		let n = n_storeys.max(1);
		if n == 1 {
			return 1;
		}
		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		// Inclusive sample of 1..=(n-1).
		1 + cfg.sample_range_usize_4d(0, n - 1, c.x, c.y, c.z, SALT_COMMERCIAL_COUNT)
	}

	/// How many shaft slots to activate when none (or few) are inbound (`1…4`).
	pub fn sample_monotower_shaft_count(confines: &Confines, noise: NoiseParams) -> usize {
		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		1 + cfg.sample_range_usize_4d(0, 4, c.x, c.y, c.z, SALT_SHAFT_COUNT)
	}

	/// Deterministic pick of `count` distinct slot indices in `0…3`.
	pub fn sample_monotower_shaft_slots(
		confines: &Confines,
		noise: NoiseParams,
		count: usize,
	) -> Vec<usize> {
		let cfg = NoiseConfig::new(noise);
		let c = monotower_noise_center(confines);
		let want = count.clamp(1, 4);
		let mut slots = Vec::new();
		let mut salt = SALT_SHAFT_PICK;
		while slots.len() < want {
			let s = cfg.sample_range_usize_4d(0, 4, c.x, c.y, c.z, salt);
			salt += 0.17;
			if !slots.contains(&s) {
				slots.push(s);
			}
		}
		slots.sort_unstable();
		slots
	}

	pub fn ring_width(&self) -> f32 {
		self.gallery_width + self.balcony_width
	}

	/// Expected count of inner-wall straight sections for active shafts.
	///
	/// Corner clears never split a side into more than one free run. Each active
	/// mid-side shaft splits its side into two.
	pub fn expected_inner_section_count(&self, active_shaft_count: usize) -> usize {
		match self.shaft_placement {
			LesHallesShaftPlacement::Corners => 4,
			LesHallesShaftPlacement::MidSides => 4 + active_shaft_count,
		}
	}

	/// Authored corner F2C clear / stall-strip buffer along each abutting wall.
	///
	/// Half the corner gallery square (`gallery_width` on a side). Consumers clamp
	/// against the gallery-inner along-half before placing clears or end walls.
	pub fn corner_clear_len(&self) -> f32 {
		(self.gallery_width * 0.5).max(2.0)
	}

	/// Pack [`Self::doors`] along a run; always tries to place at least one.
	pub fn fit_doors_on_run(&self, run_length: f32) -> Vec<LesHallesPlacedDoor> {
		fit_bays_on_run(&self.doors, run_length, true)
	}

	/// Pack exterior windows along a run (density-scaled; sparse facades allowed).
	pub fn fit_windows_on_run(&self, run_length: f32) -> Vec<LesHallesPlacedDoor> {
		fit_windows_on_run(&self.windows, self.opening_density, run_length)
	}
}

/// Split a rim budget into gallery + balcony using `stall_share`, then clamp to
/// absolute min/max depths while staying inside `ring_budget`.
fn split_ring_budget(ring_budget: f32, stall_share: f32) -> Result<(f32, f32), FitError> {
	split_ring_budget_with(ring_budget, stall_share, MIN_GALLERY_WIDTH, MAX_GALLERY_WIDTH)
}

fn split_ring_budget_livable(ring_budget: f32, stall_share: f32) -> Result<(f32, f32), FitError> {
	split_ring_budget_with(
		ring_budget,
		stall_share,
		MIN_LIVABLE_GALLERY_WIDTH,
		MAX_LIVABLE_GALLERY_WIDTH.min(MAX_GALLERY_WIDTH),
	)
}

fn split_ring_budget_monotower(ring_budget: f32, stall_share: f32) -> Result<(f32, f32), FitError> {
	split_ring_budget_with(
		ring_budget,
		stall_share,
		MIN_MONOTOWER_GALLERY_WIDTH,
		MAX_MONOTOWER_GALLERY_WIDTH.min(MAX_GALLERY_WIDTH),
	)
}

fn split_ring_budget_curtain(ring_budget: f32, stall_share: f32) -> Result<(f32, f32), FitError> {
	split_ring_budget_with(
		ring_budget,
		stall_share,
		MIN_CURTAIN_GALLERY_WIDTH,
		MAX_CURTAIN_GALLERY_WIDTH,
	)
}

fn split_ring_budget_with(
	ring_budget: f32,
	stall_share: f32,
	min_gallery: f32,
	max_gallery_abs: f32,
) -> Result<(f32, f32), FitError> {
	if ring_budget + 1e-4 < min_gallery + MIN_BALCONY_WIDTH {
		return Err(FitError::TooSmall { reason: "footprint" });
	}
	let max_gallery = max_gallery_abs.min(ring_budget - MIN_BALCONY_WIDTH).max(min_gallery);
	let target_gallery =
		(ring_budget * stall_share.clamp(0.0, 1.0)).clamp(min_gallery, max_gallery);
	let rem = (ring_budget - target_gallery).max(0.0);
	let max_balcony = MAX_BALCONY_WIDTH.min(rem).max(MIN_BALCONY_WIDTH);
	let balcony_width = rem.clamp(MIN_BALCONY_WIDTH, max_balcony);
	let gallery_width = (ring_budget - balcony_width).clamp(min_gallery, max_gallery);
	// Re-fit balcony after gallery clamp so the pair still sums to the budget when possible.
	let balcony_width = (ring_budget - gallery_width)
		.clamp(MIN_BALCONY_WIDTH, MAX_BALCONY_WIDTH.min(ring_budget - min_gallery));
	let gallery_width = (ring_budget - balcony_width)
		.clamp(min_gallery, max_gallery_abs.min(ring_budget - MIN_BALCONY_WIDTH));
	if gallery_width + balcony_width > ring_budget + 1e-3 {
		return Err(FitError::TooSmall { reason: "footprint" });
	}
	Ok((gallery_width, balcony_width))
}

/// Plan-center with \(y = 0\) so tower AABB slices share monotower noise lanes.
fn monotower_noise_center(confines: &Confines) -> bevy_math::Vec3 {
	let c = confines.center();
	bevy_math::Vec3::new(c.x, 0.0, c.z)
}

pub(crate) fn footprint_extents(confines: &Confines) -> Result<(f32, f32, f32), FitError> {
	let min = confines.bounds.min;
	let max = confines.bounds.max;
	let extent_x = max.x - min.x;
	let extent_z = max.z - min.z;
	let height = max.y - min.y;
	if !extent_x.is_finite() || !extent_z.is_finite() || !height.is_finite() {
		return Err(FitError::InvalidConfines { reason: "non_finite_bounds" });
	}
	if extent_x <= 0.0 || extent_z <= 0.0 || height <= 0.0 {
		return Err(FitError::InvalidConfines { reason: "empty_bounds" });
	}
	Ok((extent_x, extent_z, height))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn doors_catalog() -> Vec<LesHallesStallDoor> {
		vec![
			LesHallesStallDoor { door_width: 3.5, jamb_min: 0.25, allowed_error: 0.35 },
			LesHallesStallDoor { door_width: 3.0, jamb_min: 0.25, allowed_error: 0.3 },
			LesHallesStallDoor { door_width: 2.4, jamb_min: 0.2, allowed_error: 0.25 },
			LesHallesStallDoor { door_width: 1.8, jamb_min: 0.2, allowed_error: 0.2 },
		]
	}

	#[test]
	fn large_footprint_keeps_courtyard_near_half() {
		let confines = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-24.0, 0.0, -18.0),
			bevy_math::Vec3::new(24.0, 4.0, 18.0),
		));
		let params = LesHallesParameterized::sample(
			&confines,
			NoiseParams { seed: 42, ..NoiseParams::default() },
		)
		.unwrap();
		let extent_min = 36.0_f32;
		let ring = params.ring_width();
		let courtyard = extent_min - 2.0 * ring;
		let frac = courtyard / extent_min;
		assert!(
			frac > 0.35 && frac < 0.65,
			"courtyard fraction {frac:.2} (courtyard={courtyard:.1}, ring={ring:.1})"
		);
		assert!(params.gallery_width >= MIN_GALLERY_WIDTH - 1e-3);
		assert!(params.gallery_width <= MAX_GALLERY_WIDTH + 1e-3);
		assert!(params.balcony_width >= MIN_BALCONY_WIDTH - 1e-3);
		assert!(params.balcony_width <= MAX_BALCONY_WIDTH + 1e-3);
		// Stall depth should not devour the short axis.
		assert!(params.gallery_width < extent_min * 0.35);
	}

	#[test]
	fn fit_doors_places_multiple_on_long_run() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			mid_shaft_side: 3.0,
			opening_density: 0.5,
			doors: doors_catalog(),
			windows: Vec::new(),
		};
		let placed = params.fit_doors_on_run(14.0);
		assert!(placed.len() >= 2, "expected multiple doors, got {}", placed.len());
		assert!(placed.iter().all(|d| d.width >= 1.5));
	}

	#[test]
	fn fit_doors_skips_too_large_then_places_smaller() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			mid_shaft_side: 3.0,
			opening_density: 0.5,
			doors: vec![
				LesHallesStallDoor { door_width: 10.0, jamb_min: 0.5, allowed_error: 0.1 },
				LesHallesStallDoor { door_width: 2.0, jamb_min: 0.2, allowed_error: 0.2 },
			],
			windows: Vec::new(),
		};
		let placed = params.fit_doors_on_run(4.0);
		assert_eq!(placed.len(), 1);
		assert!((placed[0].width - 2.0).abs() < 0.25);
	}

	#[test]
	fn fit_windows_respects_low_density() {
		let params = LesHallesParameterized {
			gallery_width: 6.0,
			balcony_width: 4.0,
			courtyard_fraction: 0.5,
			shaft_placement: LesHallesShaftPlacement::Corners,
			mid_shaft_side: 3.0,
			opening_density: 0.05,
			doors: Vec::new(),
			windows: doors_catalog(),
		};
		assert!(params.fit_windows_on_run(20.0).is_empty());
	}

	#[test]
	fn sample_livable_biases_gallery_depth() {
		let confines = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-36.0, 0.0, -27.0),
			bevy_math::Vec3::new(36.0, 4.0, 27.0),
		));
		let params = LesHallesParameterized::sample_livable(
			&confines,
			NoiseParams { seed: 42, ..NoiseParams::default() },
		)
		.unwrap();
		assert!(
			params.gallery_width + 1e-3 >= MIN_LIVABLE_GALLERY_WIDTH,
			"gallery {:.2} < livable min",
			params.gallery_width
		);
		assert!(
			params.gallery_width <= MAX_LIVABLE_GALLERY_WIDTH + 1e-3,
			"gallery {:.2} > livable soft max",
			params.gallery_width
		);
	}

	#[test]
	fn storey_count_spans_two_to_seven_on_tall_host() {
		let short = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-36.0, 0.0, -27.0),
			bevy_math::Vec3::new(36.0, 10.0, 27.0),
		));
		let tall = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-36.0, 0.0, -27.0),
			bevy_math::Vec3::new(36.0, 35.0, 27.0),
		));
		let mut tall_seen = std::collections::BTreeSet::new();
		for seed in 0..32 {
			let noise = NoiseParams { seed, ..NoiseParams::default() };
			let n_short = LesHallesParameterized::sample_monotower_storey_count(&short, noise);
			assert!((2..=3).contains(&n_short), "10 m host seed {seed} n={n_short}");
			let n_tall = LesHallesParameterized::sample_monotower_storey_count(&tall, noise);
			assert!(
				(MIN_MONOTOWER_STOREYS..=MAX_MONOTOWER_STOREYS).contains(&n_tall),
				"35 m host seed {seed} n={n_tall}"
			);
			tall_seen.insert(n_tall);
		}
		assert!(tall_seen.len() >= 3, "tall host should vary storey count, got {tall_seen:?}");
	}

	#[test]
	fn sample_curtain_deepens_the_gallery() {
		let confines = Confines::from_bounds(bevy_math::bounding::Aabb3d::from_min_max(
			bevy_math::Vec3::new(-80.0, 0.0, -80.0),
			bevy_math::Vec3::new(80.0, 12.0, 80.0),
		));
		let params = LesHallesParameterized::sample_curtain(
			&confines,
			NoiseParams { seed: 7, ..NoiseParams::default() },
		)
		.unwrap();
		assert!(
			params.gallery_width + 1e-3 >= MIN_CURTAIN_GALLERY_WIDTH,
			"curtain gallery {:.2}",
			params.gallery_width
		);
		assert!(params.gallery_width <= MAX_CURTAIN_GALLERY_WIDTH + 1e-3);
		let courtyard = 160.0 - 2.0 * params.ring_width();
		assert!(courtyard + 1e-3 >= MIN_CURTAIN_COURTYARD, "courtyard {courtyard:.1}");
	}
}
