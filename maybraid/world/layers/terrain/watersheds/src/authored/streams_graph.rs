//! Streams graph — multi-corridor stream leaf for composition practice.
//!
//! Grows a degree-bounded [`HysteresisGraph`], collapses chains between
//! keypoints into corridors, then emits segment [`crate::primitive::node::HydroNode`]s
//! into one [`HydroComplex`] (sample-time union blend).

use crate::authored::apron::{sample_apron_rim_noise, ApronNoiseSalts};
use crate::authored::noise::{n01_freq, scale_noise_freq};
use crate::authored::stream::{
	collapse_degenerate_vertices, node_water_levels, sample_endpoint, StreamBandBudget,
	StreamParams, DEGENERATE_VERTEX_EPS, ENDPOINT_A_SALT, ENDPOINT_B_SALT,
};
use crate::primitive::backfill::HydroBackfill;
use crate::primitive::complex::HydroComplex;
use crate::primitive::node::nodes_from_polyline;
use crate::primitive::parameters::{HydroParams, TARGET_RIM_WIDTH};
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisGraph, SeededHash};
use terrain_stamps::{DownhillPair, RegionNoise};

/// Minimum channel half-width (world units); smaller budgets skip the stamp.
const MIN_HALF_WIDTH: f32 = 3.0;
/// Minimum path length (world units) along a corridor.
const MIN_PATH_LEN: f32 = 20.0;

const WIDTH_SCALE_SALT: u32 = 0x57EA_512E;
const DEGREE_SALT: u32 = 0x57EA_D6EE;

/// Extra thalweg cut on top of the freeboard bed grade. Solo streams use full
/// `StreamParams::depth` (6–10 m); graphs already absolute-grade the channel, so
/// stacking another full depth digs canyons that glue to leftover ridges.
const GRAPH_THALWEG_DEPTH_FRAC: f32 = 0.3;
const GRAPH_THALWEG_DEPTH_MAX: f32 = 2.5;

/// Authoring knobs for a streams-graph leaf.
#[derive(Debug, Clone, Copy)]
pub struct StreamsGraphParams {
	pub stream: StreamParams,
	/// Inclusive min hysteresis out-degree (`2..=4` typical).
	pub degree_min: u8,
	/// Inclusive max hysteresis out-degree.
	pub degree_max: u8,
	/// Hard cap on shared add-only rim height noise.
	pub rim_uplift_cap: f32,
}

impl Default for StreamsGraphParams {
	fn default() -> Self {
		let stream = StreamParams::default();
		let rim_uplift_cap = stream.rim.recipe_uplift_cap();
		Self { stream, degree_min: 2, degree_max: 3, rim_uplift_cap }
	}
}

fn width_scale_u01(seed: u32, leaf_min: Vec2, params: StreamParams) -> f32 {
	n01_freq(seed, WIDTH_SCALE_SALT, leaf_min, params.half_width_freq)
}

/// Pin corridor endpoints to the **local** keypoint sample so every incident
/// corridor agrees on junction \(W\), without `min`-dragging uphill heads down
/// to a foreign valley grade.
fn pin_junction_water_levels(
	corridors: &mut [CorridorPart],
	key_points: &[Vec2],
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
	sink: f32,
) {
	let sink = sink.max(0.0);
	if key_points.is_empty() || corridors.is_empty() {
		return;
	}
	let key_w: Vec<f32> = key_points
		.iter()
		.map(|p| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - sink)
		.collect();
	let n_keys = key_w.len();
	for c in corridors.iter_mut() {
		let n = c.levels.len();
		if n < 2 || c.from_key >= n_keys || c.to_key >= n_keys {
			continue;
		}
		let head = key_w[c.from_key];
		let toe = key_w[c.to_key].min(head);
		c.levels[0] = head;
		c.levels[n - 1] = toe;
		for i in 1..n {
			c.levels[i] = c.levels[i].min(c.levels[i - 1]);
		}
		c.levels[n - 1] = toe;
		for i in (1..n - 1).rev() {
			c.levels[i] = c.levels[i].min(c.levels[i - 1]).max(c.levels[n - 1]);
		}
	}
}

/// One collapsed corridor between keypoints.
#[derive(Debug, Clone)]
struct CorridorPart {
	from_key: usize,
	to_key: usize,
	path: Vec<Vec2>,
	levels: Vec<f32>,
	half_width: f32,
	freeboard: f32,
	depth: f32,
}

/// Authored streams-graph **plan**: hysteresis corridors → one hydro complex.
#[derive(Debug, Clone)]
pub struct StreamsGraph {
	pub bounds: Bounds2,
	pub seed: u32,
	pub degree: u8,
	pub half_width: f32,
	pub edge_count: usize,
	corridors: Vec<CorridorPart>,
	/// Junction / endpoint samples used while grading corridor \(W\).
	#[allow(dead_code)] // retained for junction continuity tests
	pub(crate) key_points: Vec<Vec2>,
	params: HydroParams,
	max_correction_extent: f32,
	rim_backfill: HydroBackfill,
}

impl StreamsGraph {
	/// Build a multi-corridor stream plan, or `None` when the leaf / graph is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: StreamsGraphParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let max = bounds.max;
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);

		let stream_p = params.stream;
		let width_u = width_scale_u01(seed, min, stream_p);
		let budget = StreamBandBudget::try_from_short_half(short_half, stream_p, width_u)?;
		if budget.half_width < MIN_HALF_WIDTH {
			return None;
		}

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

		let d_lo = params.degree_min.max(2).min(4);
		let d_hi = params.degree_max.max(d_lo).min(4);
		let degree = if d_lo == d_hi {
			d_lo
		} else {
			let span = (d_hi - d_lo + 1) as f32;
			let u = SeededHash::new(seed.wrapping_add(DEGREE_SALT)).unit(0);
			(d_lo + (u * span).floor() as u8).min(d_hi)
		};

		let walk = stream_p.spine.walk_config(bounds);
		let graph = HysteresisGraph::with_degree(
			degree,
			bounds,
			seed.wrapping_add(21),
			head_xz,
			toe_xz,
			&walk,
		);
		if graph.nodes.len() < 3 {
			return None;
		}

		// Channel already absolute-grades to \(W - \mathrm{freeboard}\); keep the
		// extra thalweg nick shallow so min-compose overlaps cannot canyon.
		let depth = (stream_p.depth.max(0.0) * GRAPH_THALWEG_DEPTH_FRAC)
			.min(GRAPH_THALWEG_DEPTH_MAX)
			.max(0.35);
		let apron_band = (budget.apron_half - budget.skirt_half).max(0.5);
		let apron_noise = sample_apron_rim_noise(
			&stream_p.apron,
			&stream_p.rim,
			seed,
			min,
			apron_band,
			budget.half_width,
			ApronNoiseSalts::STREAM,
		);

		let n = graph.nodes.len();
		let mut is_key = vec![false; n];
		for i in 0..n {
			let out = graph.children.get(i).map(|c| c.len()).unwrap_or(0);
			is_key[i] = i == 0 || out != 1;
		}

		let mut key_graph_idx = Vec::new();
		for i in 0..n {
			if is_key[i] {
				key_graph_idx.push(i);
			}
		}
		if key_graph_idx.len() < 2 {
			return None;
		}

		let sample_w =
			|p: Vec2| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - stream_p.water_sink.max(0.0);

		let mut corridors = Vec::new();
		for &from in &key_graph_idx {
			for &child in graph.children.get(from).into_iter().flatten() {
				let mut path_idx = vec![from, child];
				let mut cur = child;
				let mut ok = true;
				while !is_key[cur] {
					let Some(&next) = graph.children.get(cur).and_then(|c| c.first()) else {
						ok = false;
						break;
					};
					path_idx.push(next);
					cur = next;
				}
				if !ok || path_idx.len() < 2 {
					continue;
				}
				let mut path: Vec<Vec2> = path_idx.iter().map(|&k| graph.nodes[k]).collect();
				let Some(mut from_key) = key_graph_idx.iter().position(|&k| k == from) else {
					continue;
				};
				let Some(mut to_key) = key_graph_idx.iter().position(|&k| k == cur) else {
					continue;
				};
				// Grade each corridor on its own polyline (never flatten-clamp the
				// whole `graph.nodes` vec — spur tips were inheriting unrelated
				// valley \(W\) and carving ridges into hydrological nonsense).
				if sample_w(*path.last().expect("path")) > sample_w(path[0]) {
					path.reverse();
					std::mem::swap(&mut from_key, &mut to_key);
				}
				let mut levels =
					node_water_levels(&path, height_at, stream_p.water_sink, stream_p.min_drop);
				collapse_degenerate_vertices(&mut path, &mut levels, DEGENERATE_VERTEX_EPS);
				if path.len() < 2 {
					continue;
				}
				let path_len: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
				if path_len < MIN_PATH_LEN * 0.35 {
					continue;
				}

				corridors.push(CorridorPart {
					from_key,
					to_key,
					path,
					levels,
					half_width: budget.half_width,
					freeboard: stream_p.channel_freeboard.max(0.25),
					depth,
				});
			}
		}
		if corridors.is_empty() {
			return None;
		}

		let key_points = key_graph_idx.iter().map(|&i| graph.nodes[i]).collect::<Vec<_>>();

		// Shared junction \(W\) from local keypoint samples (not min-of-corridors).
		pin_junction_water_levels(&mut corridors, &key_points, height_at, stream_p.water_sink);

		let rim_w = TARGET_RIM_WIDTH;
		let apron_w = (budget.apron_half - budget.skirt_half).max(apron_band);
		let shore_amp =
			(budget.half_width.max(1.0) * stream_p.shore_indent_frac.clamp(0.0, 0.45)).max(0.01);
		let shore_freq = scale_noise_freq(
			stream_p.shore_freq.max(0.0),
			budget.half_width,
			stream_p.apron.noise_freq_power,
		);
		let boundary_noise =
			Some(RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp));
		let rim_boundary_noise = Some(apron_noise.apron.clone());
		let rim_boundary_amp = apron_noise.apron_amp;
		const RIM_BACKFILL_SALT: u32 = 0x57EA_BF11;
		let rim_backfill_params = {
			let mut p = StreamParams::rim_backfill_params(budget.half_width);
			p.freq = scale_noise_freq(p.freq, budget.half_width, stream_p.apron.noise_freq_power);
			p
		};
		// Rim backfill + shore/rim noise sit inside rim/apron — pad is band widths.
		let max_correction_extent = (rim_w + apron_w).max(0.0);
		let mut rim = stream_p.rim;
		rim.width = rim_w;
		rim.lift = stream_p.rim.lift.max(0.0);
		rim.shelf_anchor = None;
		rim.uplift_cap = params.rim_uplift_cap.max(0.0);
		let mut apron = stream_p.apron;
		apron.width = apron_w;
		let params = HydroParams {
			rim,
			apron,
			rim_height: apron_noise.rim_height,
			boundary_noise,
			rim_boundary_noise,
			shore_blend: HydroParams::recommend_shore_blend(rim_w, shore_amp),
			rim_apron_blend: HydroParams::recommend_shore_blend(
				rim_w,
				shore_amp.max(rim_boundary_amp),
			),
		};
		let rim_backfill = rim_backfill_params.sample(seed, RIM_BACKFILL_SALT);

		Some(Self {
			bounds,
			seed,
			degree,
			half_width: budget.half_width,
			edge_count: corridors.len(),
			corridors,
			key_points,
			params,
			max_correction_extent,
			rim_backfill,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, StreamsGraphParams::default(), None)
	}

	/// Hydrology nodes authored by this graph (reach segments that fit the leaf).
	pub fn hydro_nodes(&self) -> Vec<crate::primitive::node::HydroNode> {
		let mut hydrology = Vec::new();
		for corridor in &self.corridors {
			let center_depth = corridor.freeboard + corridor.depth;
			hydrology.extend(
				nodes_from_polyline(
					&corridor.path,
					&corridor.levels,
					corridor.half_width,
					center_depth,
					&self.params,
					self.max_correction_extent,
					Some(&self.rim_backfill),
				)
				.into_iter()
				.filter(|n| n.inbounds(self.bounds)),
			);
		}
		hydrology
	}

	/// Realize as a multi-corridor complex with sample-time hydro composition.
	pub fn into_complex(self) -> HydroComplex {
		HydroComplex::new(self.bounds, self.seed).with_hydro(self.hydro_nodes())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::fill::WaterSurface;

	fn slope_height(x: f32, z: f32) -> f32 {
		100.0 - 0.05 * x - 0.02 * z
	}

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 30.0, 30.0);
		assert!(StreamsGraph::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn builds_multi_edge_composed_complex() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			42,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("streams graph");
		assert!(g.edge_count >= 1);
		let compiled = g.into_complex().compile();
		assert!(compiled.has_hydro());
		assert_eq!(compiled.fills.len(), 1);
		assert!(matches!(compiled.fills[0].surface, WaterSurface::Hydro { .. }));
		Ok(())
	}

	#[test]
	fn junction_water_is_continuous() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			7,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("streams graph");
		let compiled = g.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		// Sample near each keypoint — union W should be finite and locally smooth.
		for p in &g.key_points {
			let w0 = fill.surface_level_at(p.x, p.y);
			let w1 = fill.surface_level_at(p.x + 0.5, p.y);
			assert!(
				(w0 - w1).abs() < 2.0,
				"W jump near junction {} → {}: {w0} vs {w1}",
				p,
				Vec2::new(p.x + 0.5, p.y)
			);
		}
		Ok(())
	}

	#[test]
	fn freeboard_under_wet_carve() -> anyhow::Result<()> {
		use crate::primitive::node::HydroNode;
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let params = StreamsGraphParams::default();
		let freeboard = params.stream.channel_freeboard;
		let g = StreamsGraph::from_bounds(bounds, 19, params, Some(&slope_height)).expect("graph");
		let compiled = g.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		for corridor in &g.corridors {
			assert!(corridor.path.len() >= 2);
			for window in corridor.path.windows(2) {
				let mid = window[0].lerp(window[1], 0.5);
				if !fill.inside_horizontal(mid.x, mid.y) {
					continue;
				}
				let w = fill.surface_level_at(mid.x, mid.y);
				// Freeboard is a bare hydro invariant; rim backfill may raise near shore.
				let nodes = compiled.complex.nodes_intersecting(mid);
				let h = HydroNode::elevation_blend_without_backfill(
					&nodes,
					slope_height(mid.x, mid.y),
					mid,
				);
				assert!(
					h <= w - freeboard * 0.35,
					"bare bed {h} should sit under W {w} (freeboard {freeboard}) at {mid:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn overlap_does_not_pillar_above_rim_cap() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let params = StreamsGraphParams::default();
		let rise_budget = params.stream.rim.lift.max(0.0) + params.rim_uplift_cap.max(0.0) + 4.0;
		let g = StreamsGraph::from_bounds(bounds, 3, params, Some(&slope_height)).expect("graph");
		let compiled = g.into_complex().compile();
		let mut max_rise = 0.0f32;
		for i in 0..40 {
			for j in 0..40 {
				let x = bounds.min.x + (i as f32 + 0.5) * (bounds.max.x - bounds.min.x) / 40.0;
				let z = bounds.min.y + (j as f32 + 0.5) * (bounds.max.y - bounds.min.y) / 40.0;
				let h0 = slope_height(x, z);
				let h = compiled.modify_elevation(h0, x, z);
				max_rise = max_rise.max((h - h0).max(0.0));
			}
		}
		// Soft-max of raise-only banks should stay near rim_lift + uplift cap.
		assert!(
			max_rise < rise_budget,
			"composed graph should not pillar: max_rise={max_rise} budget={rise_budget}"
		);
		Ok(())
	}

	#[test]
	fn corridor_w_tracks_local_head_not_foreign_valley() -> anyhow::Result<()> {
		// Rough terrain: the old flat-array `node_water_levels(&graph.nodes)` clamp
		// dragged spur tips down to unrelated valley \(W\).
		let height = |x: f32, z: f32| 80.0 + 25.0 * (x * 0.02).sin() + 18.0 * (z * 0.03).cos();
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let params = StreamsGraphParams::default();
		let sink = params.stream.water_sink;
		let freeboard = params.stream.channel_freeboard;
		let thalweg = (params.stream.depth * GRAPH_THALWEG_DEPTH_FRAC).min(GRAPH_THALWEG_DEPTH_MAX);
		let g = StreamsGraph::from_bounds(bounds, 11, params, Some(&height)).expect("graph");
		let compiled = g.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		for corridor in &g.corridors {
			let head = corridor.path[0];
			let head_w = corridor.levels[0];
			let head_local = height(head.x, head.y) - sink;
			assert!(
				(head_local - head_w).abs() < 0.75,
				"head W {head_w} should match local sample {head_local} at {head:?}"
			);
			for p in &corridor.path {
				let w = fill.surface_level_at(p.x, p.y);
				let h0 = height(p.x, p.y);
				let h = compiled.modify_elevation(h0, p.x, p.y);
				let floor = (w - freeboard - thalweg).min(h0);
				assert!(
					h >= floor - 0.75,
					"bed {h} below expected floor {floor} (W={w}, h0={h0}) at {p:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn fill_stays_near_channel_carve() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			42,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("graph");
		let half = g.half_width;
		let compiled = g.into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		// Outside the leaf AABB: carve support cannot reach here.
		let far = compiled.bounds.max + Vec2::splat(half * 3.0);
		assert!(
			!fill.inside_horizontal(far.x, far.y),
			"fill should not extend far past channel carve"
		);
		Ok(())
	}
}
