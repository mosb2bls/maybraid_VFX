//! Lake pocket water — [RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake).
//!
//! Authored as an ellipse [`crate::primitive::hydro::HydroPrimitive`] (`RadialBowl`) with a
//! shared complex rim/apron from \(\phi_{\mathrm{union}}\):
//! - **Water** — elliptical bowl depressed below surface \(W\) (deeper toward centroid)
//! - **Rim / apron** — one complex-wide raise-only band outside \(\phi=0\)
//!
//! Axes follow the leaf aspect, inscribed from the jittered centroid (per-edge
//! clearance), with a small noisy rotation. Leaves must still leave room for
//! rim + apron outside the water body.

mod budget;
pub(crate) mod build;
pub(crate) mod shelf;

pub use budget::LakeBandBudget;
pub use shelf::shelf_base_height;

use crate::authored::lake::build::{build_bowl, LakeBowl, LakeLayout};
use crate::authored::lake::shelf::{
	aspect_u01, planned_center as planned_center_impl, rim_width_u01, rotation_u11, shelf_levels,
	water_scale_u01,
};
use crate::primitive::backfill::RimBackfillParams;
use crate::primitive::complex::HydroComplex;
use crate::primitive::parameters::{ApronParams, RimParams};
use bevy_math::Vec2;
use procedural_common::Bounds2;

/// Authoring knobs for a Watershed lake stamp.
///
/// Grouped by what they control: footprint budget → per-leaf scales → vertical
/// levels → bowl depth → shore outline → apron outline → rim height.
#[derive(Debug, Clone, Copy)]
pub struct LakeParams {
	// ── Footprint budget ───────────────────────────────────────────────────
	/// Rim shelf width as a fraction of the leaf clearance budget.
	pub rim_frac: f32,
	/// Apron (blend-to-identity) width as a fraction of the leaf clearance budget.
	pub apron_frac: f32,
	/// Inward margin μ (world units) from cell boundary to the outer apron.
	pub mu: f32,
	/// Max centroid offset as a fraction of the smaller cell half-extent.
	pub centroid_jitter: f32,
	/// How far toward full leaf aspect (`1`) vs circular (`0`) on **large** leaves.
	pub aspect_strength: f32,
	/// Fraction of [`Self::aspect_strength`] applied on small leaves (≤
	/// [`Self::aspect_scale_ref`]). Ramps geometrically toward full strength.
	pub aspect_small: f32,
	/// Floor on the aspect draw at large scale (mild oval even when the draw is 0).
	/// Small leaves use ~0; climbs with size via [`Self::aspect_scale_ref`].
	pub aspect_floor: f32,
	/// Short leaf half-extent (world units) where anti-circle is still “small”
	/// ([`Self::aspect_small`]). Above this, strength/floor ramp up geometrically.
	pub aspect_scale_ref: f32,
	/// Max fraction of long-axis clearance the water body may claim (after rim/apron).
	pub long_axis_frac: f32,
	/// Max |rotation| of the ellipse (radians).
	pub rotation_amp: f32,

	// ── Per-leaf water scale (fraction of leftover after rim+apron claim) ───
	/// Max water radius scale (`1.0` = full leftover).
	pub water_scale: f32,
	/// Min water radius scale (paired with [`Self::water_scale`] via leaf noise).
	pub water_scale_min: f32,
	/// Spatial frequency for the water-scale draw.
	pub water_scale_freq: f32,

	// ── Per-leaf rim width ──────────────────────────────────────────────────
	/// Min rim width as a fraction of the claimed rim budget.
	pub rim_width_min: f32,
	/// Spatial frequency for the rim-width draw.
	pub rim_width_freq: f32,

	// ── Vertical levels ────────────────────────────────────────────────────
	/// How far below the shelf anchor the water surface `W` sits (world units).
	pub water_sink: f32,
	/// Per-leaf jitter on the shelf anchor height that sets `W` and rim base.
	pub shelf_amp: f32,
	/// How many pre-watershed heights to sample on a ring around the centroid
	/// (at mid-rim radius) when setting the shelf anchor. `1` = centroid only.
	pub shelf_sample_count: u8,

	// ── Bowl depth ─────────────────────────────────────────────────────────
	/// Max bowl depth at the centroid for a lake whose short water axis equals
	/// [`crate::authored::noise::NOISE_FREQ_REF_RADIUS`] (scales with water radius in build).
	pub depth: f32,
	/// Exponent on `(1 - r)` for the radial depth falloff.
	pub depth_falloff_power: f32,
	/// How much of [`Self::depth`] remains at the geometric shore (`0` = shore at
	/// `W`, `1` = flat floor at centroid depth). Softmask fade still blends past
	/// the shore.
	pub depth_shore_frac: f32,
	/// Bipolar bed-noise amplitude (world units); may raise bed above `W`.
	pub depth_noise_amp: f32,
	/// Bed-noise frequency at [`crate::authored::noise::NOISE_FREQ_REF_RADIUS`] (scaled in
	/// [`Lake::from_bounds`] by `(ref / short_water)^apron.noise_freq_power`).
	pub depth_noise_freq: f32,
	/// Extra headroom above the rim shelf for island / peninsula peaks.
	pub island_lift: f32,

	// ── Shore outline (water bowl + wet fill) — higher frequency ───────────
	/// Max bipolar shore indent/expand as a fraction of the short water axis.
	pub shore_indent_frac: f32,
	/// Shore boundary frequency at [`crate::authored::noise::NOISE_FREQ_REF_RADIUS`] (scaled geometrically
	/// in [`Lake::from_bounds`]).
	pub shore_freq: f32,

	/// Apron band + indent-noise recipe (`noise_freq_power` also scales shore/bed).
	pub apron: ApronParams,
	/// Rim band + height-noise recipe (`lift` is bank above the shelf anchor).
	pub rim: RimParams,

	/// Horizontal fill metadata pad past the bowl, as a fraction of rim width
	/// (plan `fill_radius` only — water SDF follows carve \(\phi\)).
	pub rim_bleed_frac: f32,
}

impl Default for LakeParams {
	fn default() -> Self {
		Self {
			rim_frac: 0.025,
			apron_frac: 0.35,
			mu: 12.0,
			centroid_jitter: 0.12,
			aspect_strength: 0.95,
			aspect_small: 0.22,
			aspect_floor: 0.55,
			aspect_scale_ref: 280.0,
			long_axis_frac: 0.78,
			rotation_amp: 0.55,

			water_scale: 1.0,
			water_scale_min: 0.35,
			water_scale_freq: 0.12,

			rim_width_min: 0.5,
			rim_width_freq: 0.1,

			water_sink: 0.9,
			shelf_amp: 2.0,
			shelf_sample_count: 6,

			depth: 14.0,
			depth_falloff_power: 1.35,
			depth_shore_frac: 0.0,
			depth_noise_amp: 8.0,
			depth_noise_freq: 0.016,
			island_lift: 5.5,

			shore_indent_frac: 0.18,
			shore_freq: 0.022,

			apron: ApronParams::default(),
			rim: {
				let mut rim = RimParams::default().with_visible_rim_bank();
				rim.lift = 1.25;
				rim
			},

			rim_bleed_frac: 0.5,
		}
	}
}

impl LakeParams {
	/// Rim-shore backfill recipe from the short water radius (~45% band).
	pub fn rim_backfill_params(water_radius: f32) -> RimBackfillParams {
		let mut p = RimBackfillParams::from_extent(water_radius, 0.65);
		p.freq = 0.02;
		p.amp = p.amp * 2.0;
		p
	}
}

/// Authored lake **plan**: layout metadata for one pocket-water leaf.
///
/// Realize with [`Self::into_complex`] into a [`HydroComplex`]
/// (the representation stored on terrain cells). `None` from [`Self::from_bounds`]
/// means the leaf is too small to host a lake.
#[derive(Debug, Clone)]
pub struct Lake {
	pub bounds: Bounds2,
	pub seed: u32,
	pub center: Vec2,
	/// Water half-axes in the ellipse local frame.
	pub water_radii: Vec2,
	pub rotation: f32,
	/// Characteristic (short) water half-axis.
	pub water_radius: f32,
	/// Characteristic (short) plateau half-axis.
	pub plateau_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	/// Characteristic fill half-axis (≥ [`Self::water_radius`]).
	pub fill_radius: f32,
	pub water_level: f32,
	bowl: LakeBowl,
}

impl Lake {
	/// Jittered lake centroid used by [`Self::from_bounds`] (for shelf survey).
	pub fn planned_center(bounds: Bounds2, seed: u32, params: LakeParams) -> Vec2 {
		planned_center_impl(bounds, seed, params)
	}

	/// Build a three-band lake plan, or `None` when the leaf is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: LakeParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let center = Self::planned_center(bounds, seed, params);
		let u = water_scale_u01(seed, min, params);
		let rim_u = rim_width_u01(seed, min, params);
		let asp = aspect_u01(seed, min);
		let rot = rotation_u11(seed, min);
		let budget = LakeBandBudget::try_inscribed(bounds, center, params, u, rim_u, asp, rot)?;

		let levels = shelf_levels(seed, min, center, &budget, params, height_at);
		let layout = LakeLayout { center, budget, levels };
		let bowl = build_bowl(seed, min, params, &layout);
		let fill_radius = bowl.fill_radius;

		Some(Self {
			bounds,
			seed,
			center: layout.center,
			water_radii: layout.budget.water_radii,
			rotation: layout.budget.rotation,
			water_radius: layout.budget.water_radius(),
			plateau_radius: layout.budget.plateau_radius(),
			// Authored hydro rim (may differ from the band-budget claim).
			rim_width: bowl.node.params.rim.width,
			apron_width: bowl.node.params.apron.width,
			fill_radius,
			water_level: layout.levels.water_level,
			bowl,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, LakeParams::default(), None)
	}

	/// Hydrology nodes authored by this lake (one radial bowl), if inbounds.
	pub fn hydro_nodes(&self) -> Vec<crate::primitive::node::HydroNode> {
		let node = self.bowl.node.clone();
		if node.inbounds(self.bounds) {
			vec![node]
		} else {
			Vec::new()
		}
	}

	/// Realize this plan as a sole-node [`HydroComplex`].
	pub fn into_complex(self) -> HydroComplex {
		HydroComplex::new(self.bounds, self.seed).with_hydro(self.hydro_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::authored::lake::budget::{aspect_blend, MIN_WATER_RADIUS};
	use crate::authored::noise::scale_noise_freq;
	use crate::primitive::fill::{WaterFill, WaterSurface};
	use bevy_math::Vec2;

	fn inside_fill(fill: &WaterFill, x: f32, z: f32) -> bool {
		fill.inside_horizontal(x, z)
	}

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		assert!(Lake::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn shelf_anchor_uses_ring_median_not_centroid() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let mut params = LakeParams::default();
		params.shelf_amp = 0.0;
		params.shelf_sample_count = 6;
		let center = Lake::planned_center(bounds, 11, params);
		let height = |x: f32, z: f32| {
			let d = Vec2::new(x, z).distance(center);
			if d < 2.0 {
				0.0
			} else {
				40.0
			}
		};
		let lake = Lake::from_bounds(bounds, 11, params, Some(&height)).expect("lake");
		let expected_w = 40.0 - params.water_sink;
		assert!(
			(lake.water_level - expected_w).abs() < 1e-3,
			"water_level {} should follow ring median 40, not centroid 0 (got vs {expected_w})",
			lake.water_level
		);
		Ok(())
	}

	#[test]
	fn budget_enforces_two_x_body() -> anyhow::Result<()> {
		let short_half = 160.0;
		let budget =
			LakeBandBudget::try_from_short_half(short_half, LakeParams::default(), 1.0, 1.0)
				.expect("budget");
		let leaf_short = 2.0 * short_half;
		let body_diameter = 2.0 * budget.water_radius();
		assert!(
			leaf_short + 1e-3 >= 2.0 * body_diameter,
			"leaf {leaf_short} should be ≥ 2× body {body_diameter}"
		);
		assert!(budget.rim_width > 0.0);
		assert!(budget.apron_width > 0.0);
		Ok(())
	}

	#[test]
	fn water_size_undershoot_varies_radius() -> anyhow::Result<()> {
		let short_half = 160.0;
		let params = LakeParams::default();
		let full = LakeBandBudget::try_from_short_half(short_half, params, 1.0, 1.0).expect("full");
		let mid = LakeBandBudget::try_from_short_half(short_half, params, 0.5, 1.0).expect("mid");
		let small =
			LakeBandBudget::try_from_short_half(short_half, params, 0.0, 1.0).expect("small");
		assert!(full.water_radius() > mid.water_radius());
		assert!(mid.water_radius() > small.water_radius());
		assert!((full.apron_width - small.apron_width).abs() < 1e-4);
		assert!(small.water_radius() + 1e-3 >= MIN_WATER_RADIUS);
		Ok(())
	}

	#[test]
	fn rim_width_matches_target_for_extent_fit() -> anyhow::Result<()> {
		use crate::primitive::parameters::TARGET_RIM_WIDTH;
		let short_half = 160.0;
		let params = LakeParams::default();
		let a = LakeBandBudget::try_from_short_half(short_half, params, 1.0, 1.0).expect("a");
		let b = LakeBandBudget::try_from_short_half(short_half, params, 1.0, 0.0).expect("b");
		// Fixed to authored hydro rim so correction extents match the inscription.
		anyhow::ensure!((a.rim_width - TARGET_RIM_WIDTH).abs() < 1e-3);
		anyhow::ensure!((b.rim_width - TARGET_RIM_WIDTH).abs() < 1e-3);
		anyhow::ensure!((a.water_radius() - b.water_radius()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn elongated_leaf_gets_aspect_tied_ellipse() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 200.0, 480.0);
		let mut params = LakeParams::default();
		params.aspect_strength = 1.0;
		params.rotation_amp = 0.0;
		let center = Vec2::new(100.0, 240.0);
		let budget = LakeBandBudget::try_inscribed(bounds, center, params, 1.0, 1.0, 1.0, 0.0)
			.expect("budget");
		assert!(
			budget.water_radii.y > budget.water_radii.x * 1.25,
			"tall leaf should elongate water_radii: {:?}",
			budget.water_radii
		);
		Ok(())
	}

	#[test]
	fn noise_freq_scales_geometrically_and_caps_small() -> anyhow::Result<()> {
		use crate::authored::noise::NOISE_FREQ_REF_RADIUS;
		let power = 0.5;
		let f_ref = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS, power);
		let f_small = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 0.5, power);
		let f_large = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 10.0, power);
		assert!((f_ref - 0.04).abs() < 1e-5);
		assert!(
			(f_small - f_ref).abs() < 1e-5,
			"sub-ref lakes must not exceed authored freq (linear over-harshened them)"
		);
		let expected_large = 0.04 * (0.1_f32).sqrt();
		assert!(
			(f_large - expected_large).abs() < 1e-4,
			"10× radius with √ power: got {f_large}, want ~{expected_large}"
		);
		let f_linear = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 10.0, 1.0);
		assert!(
			f_large > f_linear * 1.5,
			"√ scale should quiet large lakes less aggressively than linear"
		);
		Ok(())
	}

	#[test]
	fn aspect_blend_strengthens_on_large_leaves() -> anyhow::Result<()> {
		let params = LakeParams::default();
		let small = aspect_blend(params, params.aspect_scale_ref * 0.5, 0.5);
		let large = aspect_blend(params, params.aspect_scale_ref * 4.0, 0.5);
		assert!(
			large > small + 0.15,
			"large-leaf aspect should exceed small-leaf: small={small} large={large}"
		);
		let small_hi = aspect_blend(params, params.aspect_scale_ref * 0.5, 1.0);
		assert!(
			small_hi < params.aspect_strength * 0.45,
			"small-leaf max aspect should stay mild, got {small_hi}"
		);
		Ok(())
	}

	#[test]
	fn nearby_leaves_draw_different_water_sizes() -> anyhow::Result<()> {
		let params = LakeParams::default();
		let seed = 11u32;
		let mut radii = Vec::new();
		for i in 0..12 {
			let ox = (i as f32) * 37.0;
			let oz = (i as f32) * 29.0;
			let bounds = Bounds2::from_xz(ox, oz, ox + 320.0, oz + 320.0);
			let lake = Lake::from_bounds(bounds, seed, params, Some(&|_, _| 40.0)).expect("lake");
			radii.push(lake.water_radius);
		}
		let min_r = radii.iter().cloned().fold(f32::INFINITY, f32::min);
		let max_r = radii.iter().cloned().fold(0.0_f32, f32::max);
		assert!(
			max_r - min_r > 8.0,
			"high-freq size noise should spread water radii across nearby leaves: {radii:?}"
		);
		Ok(())
	}

	#[test]
	fn compiles_to_hydro_not_jersey_carve() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let lake =
			Lake::from_bounds(bounds, 11, LakeParams::default(), Some(&|_, _| 40.0)).expect("lake");
		let compiled = lake.into_complex().compile();
		assert!(compiled.has_hydro());
		assert_eq!(compiled.fills.len(), 1);
		assert!(matches!(compiled.fills[0].surface, WaterSurface::Hydro { .. }));
		Ok(())
	}

	#[test]
	fn bowl_below_water_level() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = LakeParams::default();
		params.depth_noise_amp = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("lake");
		let h = lake.clone().into_complex().compile().modify_elevation(
			base,
			lake.center.x,
			lake.center.y,
		);
		assert!(
			h < lake.water_level - 1.0,
			"bowl {h} should sit below surface {}",
			lake.water_level
		);
		Ok(())
	}

	#[test]
	fn rim_raises_above_water_near_shore() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = LakeParams::default();
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		params.rim.height_amp_min = 0.0;
		params.rim.height_amp_max = 0.0;
		params.shore_indent_frac = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("lake");
		let mid_r = lake.water_radius + lake.rim_width * 0.4;
		let p = lake.center + Vec2::new(mid_r, 0.0);
		let h = lake.clone().into_complex().compile().modify_elevation(base, p.x, p.y);
		assert!(h > lake.water_level + 0.1, "rim {h} should sit above water {}", lake.water_level);
		let shelf = lake.water_level + params.water_sink.max(0.0);
		// Rim backfill can add several wu of shore grit on top of bank_target.
		let rim_bf_budget = LakeParams::rim_backfill_params(lake.water_radius).amp + 2.0;
		assert!(
			h <= shelf + params.rim.lift + params.rim.height_amp_max + rim_bf_budget,
			"rim {h} should stay near shelf_anchor+rim_lift (+ rim backfill)"
		);
		Ok(())
	}

	#[test]
	fn apron_identity_far_from_basin() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = LakeParams::default();
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("lake");
		let far = lake.center + Vec2::new(lake.plateau_radius + lake.apron_width + 40.0, 0.0);
		let h = lake.clone().into_complex().compile().modify_elevation(base, far.x, far.y);
		assert!((h - base).abs() < 1e-3, "far sample should be identity: {h}");
		Ok(())
	}

	#[test]
	fn bowl_deeper_at_center() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = LakeParams::default();
		params.depth_noise_amp = 0.0;
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("lake");
		let compiled = lake.clone().into_complex().compile();
		let h_c = compiled.modify_elevation(base, lake.center.x, lake.center.y);
		let mid = lake.center + Vec2::new(lake.water_radius * 0.72, 0.0);
		let h_m = compiled.modify_elevation(base, mid.x, mid.y);
		assert!(h_c < h_m - 1.5, "center {h_c} should sit deeper than mid-bowl {h_m}");
		Ok(())
	}

	#[test]
	fn wet_inside_water_disc() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let lake = Lake::from_bounds_default(bounds, 11).expect("lake");
		let compiled = lake.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		let mid = lake.center;
		assert!(inside_fill(fill, mid.x, mid.y));
		let outside = lake.center + Vec2::new(lake.plateau_radius + lake.apron_width + 20.0, 0.0);
		assert!(!inside_fill(fill, outside.x, outside.y));
		Ok(())
	}

	#[test]
	fn fill_radius_stays_near_bowl() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let lake = Lake::from_bounds_default(bounds, 11).expect("lake");
		assert!(lake.fill_radius + 1e-3 >= lake.water_radius);
		assert!(
			lake.fill_radius <= lake.water_radius + lake.rim_width + 1e-3,
			"horizontal pad should stay on the rim"
		);
		let compiled = lake.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		assert!(inside_fill(fill, lake.center.x, lake.center.y));
		let outside = lake.center + Vec2::new(lake.plateau_radius + lake.apron_width + 5.0, 0.0);
		assert!(!inside_fill(fill, outside.x, outside.y));
		Ok(())
	}

	#[test]
	fn narrow_leaf_planned_center_does_not_panic() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 40.0, 200.0);
		let _ = Lake::planned_center(bounds, 3, LakeParams::default());
		// Narrow leaves may be too small to host a lake; survey must still be safe.
		let _ = Lake::from_bounds_default(bounds, 3);
		Ok(())
	}
}
