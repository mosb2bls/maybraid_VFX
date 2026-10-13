//! Bog pocket water — lake bowl with post-carve basin backfill.
//!
//! Not the RFC-127 §3.1.3.3 micro-lake lattice. Instead: stamp a normal lake
//! (hydro radial bowl + fill + shared apron), then attach a
//! [`crate::primitive::backfill::HydroBackfill::Basin`] on the bowl node so
//! the bed reads hummocky / islanded after water is placed.
//!
//! Basin noise itself is depth-incentive ([`BasinBackfillParams::depth_frac`]);
//! [`BogBasinFill`] chooses how aggressively that freeboard is filled / crested.

use crate::authored::lake::build::{build_bowl, LakeBowl, LakeLayout};
use crate::authored::lake::shelf::{
	aspect_u01, planned_center as planned_center_impl, rim_width_u01, rotation_u11, shelf_levels,
	water_scale_u01,
};
use crate::authored::lake::{LakeBandBudget, LakeParams};
use crate::authored::noise::scale_noise_freq;
use crate::primitive::backfill::BasinBackfillParams;
use crate::primitive::complex::HydroComplex;
use bevy_math::Vec2;
use procedural_common::Bounds2;

/// Salt offset for basin backfill noise draws.
const BASIN_BACKFILL_SALT: u32 = 0xB06_BACF1;

/// How a bog fills its basin relative to bowl freeboard / \(W\).
///
/// Converts to a [`BasinBackfillParams::depth_frac`] so backfill stays
/// depth-incentive while bog owns peaking taste.
#[derive(Debug, Clone, Copy)]
pub struct BogBasinFill {
	/// Target crest height above \(W\) at full-scale raise-only noise.
	pub peak_above_w: f32,
	/// Unit-noise fraction (`|sample| / amp`) at which the bed reaches \(W\).
	/// Higher → gentler mounds / fewer hard peaks. Clamped to `(0.05, 1]`.
	pub crest_unit: f32,
}

impl Default for BogBasinFill {
	fn default() -> Self {
		Self {
			// Short islands — frequent but not tall (~15% shorter than prior).
			peak_above_w: 0.95,
			crest_unit: 0.4,
		}
	}
}

impl BogBasinFill {
	/// Depth-incentive fraction: `amp / freeboard`.
	pub fn depth_frac(&self, freeboard: f32) -> f32 {
		let fb = freeboard.max(1e-3);
		let u = self.crest_unit.clamp(0.05, 1.0);
		let peak = self.peak_above_w.max(0.0);
		let amp = (fb / u).max(fb + peak);
		amp / fb
	}
}

/// Authoring knobs: lake footprint/bowl + basin noise + bog fill policy.
#[derive(Debug, Clone, Copy)]
pub struct BogParams {
	/// Lake bowl / apron / fill knobs (bog-tuned defaults are shallow + flat).
	pub lake: LakeParams,
	/// Spatial basin backfill knobs (`depth_frac` is set from [`Self::fill`]).
	pub basin: BasinBackfillParams,
	/// Peaking / fill policy mapped onto basin freeboard.
	pub fill: BogBasinFill,
}

impl Default for BogParams {
	fn default() -> Self {
		let mut lake = LakeParams::default();
		// Short freeboard so depth-incentive fill crests often and gently.
		lake.depth = 2.2;
		lake.depth_noise_amp = 0.65;
		lake.depth_shore_frac = 0.95;
		Self {
			lake,
			basin: BasinBackfillParams {
				// Overwritten from `fill` at stamp time.
				depth_frac: 1.0,
				freq: 0.035,
				fade: 2.5,
				octaves: 2,
				add_only: true,
			},
			fill: BogBasinFill::default(),
		}
	}
}

/// Authored bog **plan**: lake-shaped layout + basin backfill on the bowl node.
///
/// Realize with [`Self::into_complex`] into a [`HydroComplex`].
/// `None` from [`Self::from_bounds`] means the leaf is too small.
#[derive(Debug, Clone)]
pub struct Bog {
	pub bounds: Bounds2,
	pub seed: u32,
	pub center: Vec2,
	pub water_radii: Vec2,
	pub rotation: f32,
	pub water_radius: f32,
	pub plateau_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub fill_radius: f32,
	pub water_level: f32,
	bowl: LakeBowl,
}

impl Bog {
	pub fn planned_center(bounds: Bounds2, seed: u32, params: BogParams) -> Vec2 {
		planned_center_impl(bounds, seed, params.lake)
	}

	/// Build a lake bowl + basin backfill plan, or `None` when the leaf is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: BogParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let center = Self::planned_center(bounds, seed, params);
		let lake_p = params.lake;
		let u = water_scale_u01(seed, min, lake_p);
		let rim_u = rim_width_u01(seed, min, lake_p);
		let asp = aspect_u01(seed, min);
		let rot = rotation_u11(seed, min);
		let budget = LakeBandBudget::try_inscribed(bounds, center, lake_p, u, rim_u, asp, rot)?;

		let levels = shelf_levels(seed, min, center, &budget, lake_p, height_at);
		let layout = LakeLayout { center, budget, levels };
		let mut bowl = build_bowl(seed, min, lake_p, &layout);
		let fill_radius = bowl.fill_radius;

		let short_water = layout.budget.water_radius();
		let basin_freq =
			scale_noise_freq(params.basin.freq, short_water, lake_p.apron.noise_freq_power);
		// Cover jittered_depth high end (~1.35× authored).
		let freeboard = lake_p.depth.max(0.0) * 1.35;
		let basin = BasinBackfillParams { freq: basin_freq, ..params.basin }
			.near_surface(freeboard, params.fill.peak_above_w, params.fill.crest_unit)
			.sample_over_freeboard(freeboard, seed, BASIN_BACKFILL_SALT);

		// Basin replaces the lake's abundant rim backfill for bog peaking.
		bowl.node.backfill = Some(basin);

		Some(Self {
			bounds,
			seed,
			center: layout.center,
			water_radii: layout.budget.water_radii,
			rotation: layout.budget.rotation,
			water_radius: layout.budget.water_radius(),
			plateau_radius: layout.budget.plateau_radius(),
			rim_width: layout.budget.rim_width,
			apron_width: layout.budget.apron_width,
			fill_radius,
			water_level: layout.levels.water_level,
			bowl,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, BogParams::default(), None)
	}

	/// Hydrology nodes authored by this bog (lake bowl + basin backfill), if inbounds.
	pub fn hydro_nodes(&self) -> Vec<crate::primitive::node::HydroNode> {
		let node = self.bowl.node.clone();
		if node.inbounds(self.bounds) {
			vec![node]
		} else {
			Vec::new()
		}
	}

	/// Realize this plan as a sole-node complex (basin rides on the node).
	pub fn into_complex(self) -> HydroComplex {
		HydroComplex::new(self.bounds, self.seed).with_hydro(self.hydro_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::backfill::HydroBackfill;
	use crate::primitive::fill::WaterSurface;
	use crate::primitive::node::HydroNode;

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		assert!(Bog::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn from_bounds_hydro_plus_basin_backfill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog =
			Bog::from_bounds(bounds, 11, BogParams::default(), Some(&|_, _| 40.0)).expect("bog");
		let compiled = bog.clone().into_complex().compile();
		assert!(compiled.has_hydro());
		assert!(!compiled.fills.is_empty());
		assert!(matches!(compiled.fills[0].surface, WaterSurface::Hydro { .. }));
		let node = &compiled.complex.hydrology[0];
		anyhow::ensure!(
			matches!(node.backfill, Some(HydroBackfill::Basin(_))),
			"bog node should carry basin backfill"
		);

		let base = 40.0;
		let nodes = [&compiled.complex.hydrology[0]];
		let p_in = bog.center;
		let outside = bog.center + Vec2::new(bog.plateau_radius + bog.apron_width + 40.0, 0.0);
		let h_bare = HydroNode::elevation_blend_without_backfill(&nodes, base, p_in);
		let h_full = HydroNode::elevation_blend(&nodes, base, p_in);
		anyhow::ensure!(
			h_full > h_bare + 0.05,
			"basin backfill should raise interior: bare={h_bare} full={h_full}"
		);
		let h_out = compiled.modify_elevation(base, outside.x, outside.y);
		anyhow::ensure!((h_out - base).abs() < 1e-3, "far field identity: {h_out}");
		Ok(())
	}

	#[test]
	fn bog_keeps_water_fill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog = Bog::from_bounds_default(bounds, 19).expect("bog");
		assert_eq!(bog.clone().into_complex().compile().fills.len(), 1);
		Ok(())
	}

	#[test]
	fn fill_policy_sets_depth_frac_from_freeboard() -> anyhow::Result<()> {
		let params = BogParams::default();
		let freeboard = params.lake.depth * 1.35;
		let frac = params.fill.depth_frac(freeboard);
		let amp =
			BasinBackfillParams { depth_frac: frac, ..params.basin }.amp_for_freeboard(freeboard);
		let u = params.fill.crest_unit;
		assert!(
			(amp - (freeboard / u).max(freeboard + params.fill.peak_above_w)).abs() < 1e-3,
			"amp {amp} should match fill policy for freeboard {freeboard}"
		);
		// Milder than a tall-spike recipe; still depth-incentive.
		assert!(frac <= 2.5, "expected moderate depth_frac, got {frac}");
		assert!(frac > 1.5, "expected enough fill to crest often, got {frac}");
		Ok(())
	}

	#[test]
	fn some_interior_samples_crest_above_water() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = BogParams::default();
		params.lake.rotation_amp = 0.0;
		params.lake.aspect_strength = 0.0;
		params.lake.aspect_floor = 0.0;
		params.lake.depth_noise_amp = 0.0;
		let bog = Bog::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("bog");
		let compiled = bog.clone().into_complex().compile();

		let mut crested = 0usize;
		let mut samples = 0usize;
		let mut max_above = 0.0_f32;
		for &frac in &[0.15_f32, 0.35, 0.55, 0.7] {
			for i in 0..24 {
				let ang = i as f32 * std::f32::consts::TAU / 24.0;
				let p = bog.center + Vec2::new(ang.cos(), ang.sin()) * (bog.water_radius * frac);
				let h = compiled.modify_elevation(base, p.x, p.y);
				samples += 1;
				if h > bog.water_level + 0.15 {
					crested += 1;
					max_above = max_above.max(h - bog.water_level);
				}
			}
		}
		assert!(crested >= 3, "expected some island crests above W (crested={crested}/{samples})");
		// Mild fill: peaks should not tower far above W.
		assert!(max_above < 4.0, "crests should stay modest above W, got +{max_above}");
		Ok(())
	}
}
