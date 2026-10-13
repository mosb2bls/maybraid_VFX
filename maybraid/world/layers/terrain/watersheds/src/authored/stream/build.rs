//! Assemble stream corridor as hydrology reach-segment nodes.

use crate::authored::apron::{jittered_depth, sample_apron_rim_noise, ApronNoiseSalts};
use crate::authored::noise::scale_noise_freq;
use crate::authored::stream::{StreamBandBudget, StreamParams};
use crate::primitive::backfill::HydroBackfill;
use crate::primitive::parameters::{HydroParams, TARGET_RIM_WIDTH};
use bevy_math::Vec2;
use terrain_stamps::RegionNoise;

const DEPTH_SALT: u32 = 0x57EA_DE07;
const RIM_BACKFILL_SALT: u32 = 0x57EA_BF11;

/// Laid-out stream geometry ready for depression / apron / fill construction.
pub(crate) struct StreamLayout {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub budget: StreamBandBudget,
}

/// Stream-specific stamp: one corridor as hydrology nodes.
#[derive(Debug, Clone)]
pub(crate) struct StreamCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub center_depth: f32,
	pub params: HydroParams,
	pub max_correction_extent: f32,
	pub rim_backfill: HydroBackfill,
}

pub(crate) fn build_corridor(
	seed: u32,
	anchor: Vec2,
	params: StreamParams,
	layout: &StreamLayout,
) -> StreamCorridor {
	let path = layout.path.clone();
	let levels = layout.levels.clone();
	let half_w = layout.budget.half_width;
	let skirt_w = layout.budget.skirt_half;
	let apron_w = layout.budget.apron_half;

	let depth = jittered_depth(seed, DEPTH_SALT, anchor, params.depth, 0.7, 0.6);
	let freeboard = params.channel_freeboard.max(0.25);
	let center_depth = freeboard + depth;

	let apron_band = (apron_w - skirt_w).max(0.5);
	let apron_noise = sample_apron_rim_noise(
		&params.apron,
		&params.rim,
		seed,
		anchor,
		apron_band,
		half_w,
		ApronNoiseSalts::STREAM,
	);

	let rim_w = TARGET_RIM_WIDTH;
	let apron_width = (apron_w - skirt_w).max(apron_band);
	let shore_amp = (half_w.max(1.0) * params.shore_indent_frac.clamp(0.0, 0.45)).max(0.01);
	let shore_freq =
		scale_noise_freq(params.shore_freq.max(0.0), half_w, params.apron.noise_freq_power);
	let boundary_noise = Some(RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp));
	let rim_boundary_noise = Some(apron_noise.apron.clone());
	let rim_boundary_amp = apron_noise.apron_amp;
	let rim_backfill_params = {
		let mut p = StreamParams::rim_backfill_params(half_w);
		p.freq = scale_noise_freq(p.freq, half_w, params.apron.noise_freq_power);
		p
	};
	// Rim backfill + shore/rim noise sit inside rim/apron — pad is band widths.
	let max_correction_extent = (rim_w + apron_width).max(0.0);
	let mut rim = params.rim;
	rim.width = rim_w;
	rim.lift = params.rim.lift.max(0.0);
	rim.shelf_anchor = None;
	rim.uplift_cap = params.rim.recipe_uplift_cap();
	let mut apron = params.apron;
	apron.width = apron_width;
	let hydro_params = HydroParams {
		rim,
		apron,
		rim_height: apron_noise.rim_height,
		boundary_noise,
		rim_boundary_noise,
		shore_blend: HydroParams::recommend_shore_blend(rim_w, shore_amp),
		rim_apron_blend: HydroParams::recommend_shore_blend(rim_w, shore_amp.max(rim_boundary_amp)),
	};
	let rim_backfill = rim_backfill_params.sample(seed, RIM_BACKFILL_SALT);

	StreamCorridor {
		path,
		levels,
		half_width: half_w,
		center_depth,
		params: hydro_params,
		max_correction_extent,
		rim_backfill,
	}
}
