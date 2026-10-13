//! Stream pocket water — [RFC-127 §3.1.3.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3132-stream).
//!
//! Distance-to-polyline bands (centerline → edge):
//! - **Thalweg** — deepest cut along the path
//! - **Wet channel** — floor near graded surface \(W(s)\)
//! - **Skirt / rim** — bank lift slightly above \(W(s)\)
//!
//! Fill / channel grade is piecewise along path nodes (segment lerp + pitch blend).

mod build;
mod path;

pub(crate) use path::{
	collapse_degenerate_vertices, node_water_levels, sample_endpoint, DEGENERATE_VERTEX_EPS,
	ENDPOINT_A_SALT, ENDPOINT_B_SALT,
};

use crate::authored::noise::n01_freq;
use crate::authored::stream::build::{build_corridor, StreamCorridor, StreamLayout};
use crate::primitive::backfill::RimBackfillParams;
use crate::primitive::complex::HydroComplex;
use crate::primitive::parameters::{ApronParams, RimParams};
use bevy_math::Vec2;
use procedural_common::Bounds2;
use terrain_stamps::{DownhillPair, HysteresisSpine};

/// Minimum channel half-width (world units); smaller budgets skip the stamp.
const MIN_HALF_WIDTH: f32 = 3.0;
/// Minimum path length (world units).
const MIN_PATH_LEN: f32 = 24.0;

const WIDTH_SCALE_SALT: u32 = 0x57EA_512E;

/// Authoring knobs for a Watershed stream stamp.
#[derive(Debug, Clone, Copy)]
pub struct StreamParams {
	/// Channel half-width as a fraction of the leaf short half-extent.
	pub half_width_frac: f32,
	/// Min half-width scale (`1.0` = full claimed width).
	pub half_width_scale_min: f32,
	/// Max half-width scale.
	pub half_width_scale: f32,
	pub half_width_freq: f32,

	/// Thalweg half-width as a fraction of channel half-width.
	pub thalweg_ratio: f32,
	/// Skirt / bank width beyond the wet channel (world units fraction of half-width).
	pub skirt_extra_frac: f32,
	/// Soft apron past the skirt (blend to identity), as a fraction of half-width.
	pub apron_frac: f32,
	/// Inward margin μ from cell boundary when placing endpoints.
	pub mu: f32,

	/// How far below endpoint terrain the water surface sits.
	pub water_sink: f32,
	/// Minimum forced drop between consecutive nodes when a segment is flat/uphill.
	pub min_drop: f32,
	/// Path-distance radius for inbound/outbound pitch blending at vertices.
	/// When `0`, uses ~half a hysteresis step (from [`Self::spine`]).
	pub node_blend: f32,

	/// Thalweg cut depth (world units).
	pub depth: f32,
	pub depth_noise_amp: f32,

	/// Shore / bank outline amplitude as a fraction of half-width (terrain mods only).
	pub shore_indent_frac: f32,
	pub shore_freq: f32,

	/// Apron band + indent-noise recipe.
	pub apron: ApronParams,
	/// Rim band + height-noise recipe (`lift` is bank above \(W\)).
	pub rim: RimParams,

	/// How far below the water surface \(W\) the channel floor grade sits.
	///
	/// Keeps the carved bed under \(W\) so wet columns stay open under the
	/// free-surface half-space (see [`crate::primitive::fill::WaterFill`]).
	pub channel_freeboard: f32,

	/// Hysteresis spine walk (step / snap) — uses Stamp defaults when left at default.
	pub spine: HysteresisSpine,
}

impl Default for StreamParams {
	fn default() -> Self {
		Self {
			half_width_frac: 0.08,
			half_width_scale_min: 0.55,
			half_width_scale: 1.0,
			half_width_freq: 0.1,

			thalweg_ratio: 0.35,
			skirt_extra_frac: 0.85,
			apron_frac: 1.4,
			mu: 10.0,

			water_sink: 0.7,
			min_drop: 0.75,
			node_blend: 0.0,

			depth: 8.0,
			depth_noise_amp: 1.5,

			shore_indent_frac: 0.2,
			shore_freq: 0.04,

			apron: ApronParams::default(),
			rim: RimParams::default().with_visible_rim_bank(),

			channel_freeboard: 2.0,

			spine: HysteresisSpine::default(),
		}
	}
}

impl StreamParams {
	/// Rim-shore backfill recipe from channel half-width (~80% band, low freq).
	pub fn rim_backfill_params(half_width: f32) -> RimBackfillParams {
		let mut p = RimBackfillParams::from_extent(half_width, 0.65);
		p.freq = 0.02;
		p.amp = p.amp * 2.1;
		p
	}
}

/// Band widths derived from a leaf short half-extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamBandBudget {
	pub half_width: f32,
	pub thalweg_half: f32,
	pub skirt_half: f32,
	pub apron_half: f32,
	pub mu: f32,
}

impl StreamBandBudget {
	/// Returns `None` when the leaf cannot host a meaningful stream corridor.
	pub fn try_from_short_half(
		short_half: f32,
		params: StreamParams,
		width_u01: f32,
	) -> Option<Self> {
		let s = short_half.max(0.0);
		let mu = params.mu.min(s * 0.25).max(0.0);
		let available = (s - mu).max(0.0);
		if available < MIN_HALF_WIDTH * 3.0 {
			return None;
		}
		let claim = (available * params.half_width_frac.clamp(0.03, 0.22))
			.max(MIN_HALF_WIDTH)
			.min(available * 0.28);
		let hi = params.half_width_scale.clamp(0.2, 1.0);
		let lo = params.half_width_scale_min.clamp(0.2, hi);
		let half = claim * (lo + (hi - lo) * width_u01.clamp(0.0, 1.0));
		if half < MIN_HALF_WIDTH {
			return None;
		}
		let thalweg = (half * params.thalweg_ratio.clamp(0.15, 0.7)).max(1.0);
		let skirt = half * (1.0 + params.skirt_extra_frac.max(0.2));
		let apron = skirt + half * params.apron_frac.max(0.4);
		Some(Self {
			half_width: half,
			thalweg_half: thalweg,
			skirt_half: skirt,
			apron_half: apron,
			mu,
		})
	}
}

fn width_scale_u01(seed: u32, leaf_min: Vec2, params: StreamParams) -> f32 {
	n01_freq(seed, WIDTH_SCALE_SALT, leaf_min, params.half_width_freq)
}

/// Authored stream **plan**: layout metadata for one pocket-water leaf.
///
/// Realize with [`Self::into_complex`] into a [`HydroComplex`].
/// `None` from [`Self::from_bounds`] means the leaf / path is too small.
#[derive(Debug, Clone)]
pub struct Stream {
	pub bounds: Bounds2,
	pub seed: u32,
	pub path: Vec<Vec2>,
	/// Per-vertex water surface elevations along [`Self::path`].
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub thalweg_half: f32,
	pub skirt_half: f32,
	pub head_water: f32,
	pub toe_water: f32,
	corridor: StreamCorridor,
}

impl Stream {
	/// Build a graded stream plan, or `None` when the leaf / path is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: StreamParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let max = bounds.max;
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);

		let width_u = width_scale_u01(seed, min, params);
		let budget = StreamBandBudget::try_from_short_half(short_half, params, width_u)?;

		let inset = budget.apron_half.max(budget.mu);
		let lo = min + Vec2::splat(inset);
		let hi = max - Vec2::splat(inset);
		if lo.x >= hi.x || lo.y >= hi.y {
			return None;
		}

		let a0 = sample_endpoint(seed, ENDPOINT_A_SALT, lo, hi);
		let b0 = sample_endpoint(seed, ENDPOINT_B_SALT, lo, hi);
		if a0.distance(b0) < MIN_PATH_LEN * 0.5 {
			return None;
		}

		let (head_xz, _, toe_xz, _) = DownhillPair::order(a0, b0, height_at);
		let path = params.spine.build(bounds, seed.wrapping_add(21), head_xz, toe_xz);
		if path.len() < 2 {
			return None;
		}
		let path_len: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
		if path_len < MIN_PATH_LEN {
			return None;
		}

		let mut path = path;
		let mut levels = node_water_levels(&path, height_at, params.water_sink, params.min_drop);
		collapse_degenerate_vertices(&mut path, &mut levels, DEGENERATE_VERTEX_EPS);
		if path.len() < 2 {
			return None;
		}
		let head_water = levels.first().copied().unwrap_or(0.0);
		let toe_water = levels.last().copied().unwrap_or(head_water);
		let layout = StreamLayout { path, levels, budget };
		let corridor = build_corridor(seed, min, params, &layout);

		Some(Self {
			bounds,
			seed,
			path: layout.path,
			levels: layout.levels,
			half_width: layout.budget.half_width,
			thalweg_half: layout.budget.thalweg_half,
			skirt_half: layout.budget.skirt_half,
			head_water,
			toe_water,
			corridor,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, StreamParams::default(), None)
	}

	/// Hydrology nodes authored by this stream (reach segments that fit the leaf).
	pub fn hydro_nodes(&self) -> Vec<crate::primitive::node::HydroNode> {
		crate::primitive::node::nodes_from_polyline(
			&self.corridor.path,
			&self.corridor.levels,
			self.corridor.half_width,
			self.corridor.center_depth,
			&self.corridor.params,
			self.corridor.max_correction_extent,
			Some(&self.corridor.rim_backfill),
		)
		.into_iter()
		.filter(|n| n.inbounds(self.bounds))
		.collect()
	}

	/// Realize this plan as a sole-corridor [`HydroComplex`].
	pub fn into_complex(self) -> HydroComplex {
		HydroComplex::new(self.bounds, self.seed).with_hydro(self.hydro_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::fill::WaterSurface;

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		assert!(Stream::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn graded_fill_decreases_along_path() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let stream = Stream::from_bounds(bounds, 42, StreamParams::default(), Some(&height))
			.expect("stream");
		assert!(stream.head_water > stream.toe_water);
		let compiled = stream.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		assert!(matches!(fill.surface, WaterSurface::Hydro { .. }));
		let head = *stream.path.first().expect("path head");
		let toe = *stream.path.last().expect("path toe");
		let w_head = fill.surface_level_at(head.x, head.y);
		let w_toe = fill.surface_level_at(toe.x, toe.y);
		assert!(w_head > w_toe + 0.2, "graded fill should drop along path: {w_head} → {w_toe}");
		assert!(fill.inside_horizontal(head.x, head.y));
		let far = Vec2::new(bounds.min.x - 80.0, bounds.min.y - 80.0);
		assert!(!fill.inside_horizontal(far.x, far.y));
		Ok(())
	}

	#[test]
	fn fill_tracks_channel_carve() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let params = StreamParams::default();
		let stream = Stream::from_bounds(bounds, 42, params, Some(&height)).expect("stream");
		let compiled = stream.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		let mid = stream.path[0].lerp(stream.path[1], 0.5);
		assert!(fill.inside_horizontal(mid.x, mid.y));
		let far = Vec2::new(mid.x, mid.y + stream.half_width * 3.0);
		assert!(!fill.inside_horizontal(far.x, far.y));
		Ok(())
	}

	#[test]
	fn channel_bed_below_w_along_segments() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.04 * x - 0.02 * z;
		let params = StreamParams::default();
		let freeboard = params.channel_freeboard;
		let stream = Stream::from_bounds(bounds, 19, params, Some(&height)).expect("stream");
		let compiled = stream.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		assert!(stream.path.len() >= 2);
		for window in stream.path.windows(2) {
			let mid = window[0].lerp(window[1], 0.5);
			let w = fill.surface_level_at(mid.x, mid.y);
			let h = compiled.modify_elevation(height(mid.x, mid.y), mid.x, mid.y);
			assert!(
				h < w - freeboard * 0.5,
				"mid-segment bed {h} should sit under W {w} (freeboard {freeboard})"
			);
			assert!(
				fill.wet_y_span_at(mid.x, mid.y, h).is_some(),
				"exact fill should have volume mid-segment"
			);
		}
		Ok(())
	}

	#[test]
	fn fill_tracks_piecewise_node_samples() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, _z: f32| {
			if x < 200.0 {
				120.0 - 0.2 * x
			} else {
				80.0
			}
		};
		let mut params = StreamParams::default();
		params.min_drop = 0.0;
		params.water_sink = 0.0;
		let stream = Stream::from_bounds(bounds, 7, params, Some(&height)).expect("stream");
		assert_eq!(stream.path.len(), stream.levels.len());
		let compiled = stream.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		for (p, &w) in stream.path.iter().zip(stream.levels.iter()) {
			let got = fill.surface_level_at(p.x, p.y);
			assert!((got - w).abs() < 0.4, "node ({}, {}) W={got} expected {w}", p.x, p.y);
		}
		if stream.path.len() >= 2 {
			let a = stream.path[0];
			let b = stream.path[1];
			let mid = a.lerp(b, 0.5);
			let w_mid = fill.surface_level_at(mid.x, mid.y);
			let lo = stream.levels[0].min(stream.levels[1]);
			let hi = stream.levels[0].max(stream.levels[1]);
			assert!(w_mid >= lo - 0.2 && w_mid <= hi + 0.2);
		}
		Ok(())
	}

	#[test]
	fn hydro_complex_has_rim_apron_identity_far_away() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let params = StreamParams::default();
		assert!(params.rim.height_amp_max >= 10.0);
		assert!(params.apron.indent_frac_max > params.apron.indent_frac_min);
		let stream = Stream::from_bounds(bounds, 42, params, Some(&height)).expect("stream");
		let compiled = stream.clone().into_complex().compile();
		assert!(compiled.has_hydro());
		let far = Vec2::new(bounds.min.x - 120.0, bounds.min.y - 120.0);
		let h0 = height(far.x, far.y);
		let h1 = compiled.modify_elevation(h0, far.x, far.y);
		assert!((h1 - h0).abs() < 1e-3);
		Ok(())
	}
}
