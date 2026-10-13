//! Leaf helpers that draw apron indent + rim height from primitive recipes.
//!
//! Runtime bands live on [`crate::primitive::parameters::HydroParams`]. These
//! helpers only sample [`ApronParams`] / [`RimParams`] noise ranges at construct.

use crate::authored::noise::{n01_at, scale_noise_freq};
use crate::primitive::parameters::{ApronParams, RimParams};
use bevy_math::Vec2;
use terrain_stamps::RegionNoise;

/// Deterministic salts for [`sample_apron_rim_noise`].
#[derive(Debug, Clone, Copy)]
pub struct ApronNoiseSalts {
	pub indent_amp: u32,
	pub indent_freq: u32,
	pub rim_amp: u32,
	pub rim_freq: u32,
}

impl ApronNoiseSalts {
	/// Lake leaf salt family (`0x1A7E_*`).
	pub const LAKE: Self = Self {
		indent_amp: 0x1A7E_A70A,
		indent_freq: 0x1A7E_AF7E,
		rim_amp: 0x1A7E_A17A,
		rim_freq: 0x1A7E_F7E9,
	};

	/// Stream leaf salt family (`0x57EA_*`).
	pub const STREAM: Self = Self {
		indent_amp: 0x57EA_A70A,
		indent_freq: 0x57EA_AF7E,
		rim_amp: 0x57EA_A17A,
		rim_freq: 0x57EA_F7E9,
	};
}

/// Drawn apron (ring→apron) boundary + add-only rim height noises for one leaf bake.
#[derive(Debug, Clone)]
pub struct ApronRimNoise {
	/// Spatial warp for [`crate::primitive::parameters::HydroParams::rim_boundary_noise`].
	pub apron: RegionNoise,
	/// Peak indent amplitude (broadphase pad / blend sizing).
	pub apron_amp: f32,
	pub rim_height: RegionNoise,
}

/// Sample apron indent + rim height noises at `anchor`.
///
/// `apron_width_for_amp` is the width the indent fraction multiplies
/// (lake: apron width; stream: apron−skirt band). `scale_radius` is the
/// characteristic size for [`scale_noise_freq`].
pub fn sample_apron_rim_noise(
	apron: &ApronParams,
	rim: &RimParams,
	seed: u32,
	anchor: Vec2,
	apron_width_for_amp: f32,
	scale_radius: f32,
	salts: ApronNoiseSalts,
) -> ApronRimNoise {
	let apron_frac_lo = apron.indent_frac_min.min(apron.indent_frac_max).clamp(0.0, 0.5);
	let apron_frac_hi = apron.indent_frac_min.max(apron.indent_frac_max).clamp(0.0, 0.5);
	let apron_freq_lo = apron.freq_min.min(apron.freq_max).max(0.0);
	let apron_freq_hi = apron.freq_min.max(apron.freq_max).max(0.0);
	let apron_indent_frac =
		apron_frac_lo + (apron_frac_hi - apron_frac_lo) * n01_at(seed, salts.indent_amp, anchor);
	let apron_amp = (apron_width_for_amp.max(0.0) * apron_indent_frac).max(0.01);
	let apron_freq_authored =
		apron_freq_lo + (apron_freq_hi - apron_freq_lo) * n01_at(seed, salts.indent_freq, anchor);
	let apron_freq = scale_noise_freq(apron_freq_authored, scale_radius, apron.noise_freq_power);
	let apron_noise = RegionNoise::from_seed(seed.wrapping_add(6), apron_freq, apron_amp);

	let rim_amp_lo = rim.height_amp_min.min(rim.height_amp_max).max(0.0);
	let rim_amp_hi = rim.height_amp_min.max(rim.height_amp_max).max(0.0);
	let rim_freq_lo = rim.height_freq_min.min(rim.height_freq_max).max(0.0);
	let rim_freq_hi = rim.height_freq_min.max(rim.height_freq_max).max(0.0);
	let rim_height_amp =
		rim_amp_lo + (rim_amp_hi - rim_amp_lo) * n01_at(seed, salts.rim_amp, anchor);
	let rim_freq_authored =
		rim_freq_lo + (rim_freq_hi - rim_freq_lo) * n01_at(seed, salts.rim_freq, anchor);
	let rim_height_freq = scale_noise_freq(rim_freq_authored, scale_radius, apron.noise_freq_power);
	let rim_height = RegionNoise::from_seed(seed.wrapping_add(7), rim_height_freq, rim_height_amp);

	ApronRimNoise { apron: apron_noise, apron_amp, rim_height }
}

/// Per-leaf depth scale: `depth * (lo + span * u01)`.
pub fn jittered_depth(seed: u32, salt: u32, anchor: Vec2, depth: f32, lo: f32, span: f32) -> f32 {
	depth * (lo + span * n01_at(seed, salt, anchor))
}
