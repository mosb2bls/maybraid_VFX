//! Deterministic unit noise for Watershed anchors via [`procedural_common::SeededHash`].

use bevy_math::Vec2;
use procedural_common::SeededHash;

/// Authored `*_freq` knobs are defined at this characteristic water radius.
/// [`scale_noise_freq`] applies a **geometric** (power-law) scale
/// `(ref / radius)^power` — the geometric mean of constant-wavelength and
/// constant-lobe-count when `power = 0.5`.
pub const NOISE_FREQ_REF_RADIUS: f32 = 80.0;

/// Scale an authored frequency from [`NOISE_FREQ_REF_RADIUS`] to `radius`.
///
/// ```text
/// f = f_ref * (ref / radius)^power
/// ```
///
/// `power = 0.5` is the geometric mean of constant wavelength (`^0`) and
/// constant lobe count (`^1`). Linear `power = 1` over-harshens small features
/// and over-smooths the path between bands; √ tracks perceived roughness better.
///
/// Sub-ref radii still clamp the scale to ≤ 1 so small features never exceed the
/// authored reference roughness.
pub fn scale_noise_freq(authored_at_ref: f32, radius: f32, power: f32) -> f32 {
	let r = radius.max(1.0);
	let ratio = NOISE_FREQ_REF_RADIUS / r;
	let scale = ratio.powf(power.clamp(0.15, 2.0)).min(1.0);
	(authored_at_ref.max(0.0) * scale).clamp(1.0e-4, 0.14)
}

/// Unit sample in `[0, 1)` from `seed` ⊕ `salt` at integer lattice coords.
pub fn n01(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	SeededHash::new(seed.wrapping_add(salt)).unit_i32(ix, iz)
}

/// Unit sample in `[0, 1)` keyed by a world point (bit-cast coords).
pub fn n01_at(seed: u32, salt: u32, p: Vec2) -> f32 {
	n01(seed, salt, p.x.to_bits() as i32, p.y.to_bits() as i32)
}

/// High-frequency spatial unit sample.
///
/// Uses a fine world lattice so nearby leaves decorrelate, mixed with the
/// point's bit identity so two leaves on the same lattice cell still differ.
/// Returns a single hash draw (do not average multiple unit samples — that
/// shrinks variance toward ½ and flattens lake sizes).
pub fn n01_freq(seed: u32, salt: u32, p: Vec2, freq: f32) -> f32 {
	let f = freq.max(1.0e-3);
	let ix = (p.x * f).floor() as i32;
	let iz = (p.y * f).floor() as i32;
	// Second, higher octave for extra spatial scrambling.
	let jx = (p.x * f * 4.17).floor() as i32;
	let jz = (p.y * f * 4.17).floor() as i32;
	let ax = ix
		.wrapping_mul(374_761_393)
		.wrapping_add(jx.wrapping_mul(668_265_263))
		.wrapping_add(p.x.to_bits() as i32);
	let az = iz
		.wrapping_mul(127_412_617)
		.wrapping_add(jz.wrapping_mul(224_682_251))
		.wrapping_add(p.y.to_bits() as i32);
	n01(seed, salt, ax, az)
}

/// Signed sample in `[-1, 1)`.
pub fn n11(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	n01(seed, salt, ix, iz) * 2.0 - 1.0
}

pub fn n11_at(seed: u32, salt: u32, p: Vec2) -> f32 {
	n01_at(seed, salt, p) * 2.0 - 1.0
}
