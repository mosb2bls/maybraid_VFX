//! Per-node rim / apron parameters for hydro correction.
//!
//! Every [`crate::primitive::node::HydroNode`] carries [`HydroParams`]. Leaf
//! stamps draw noise from the nested recipe fields, then set band widths /
//! [`HydroParams::rim_height`] before emit.

pub mod apron;
pub mod rim;

pub use apron::ApronParams;
pub use rim::RimParams;

use bevy_math::Vec2;
use terrain_stamps::RegionNoise;

/// Which watershed correction pass to apply at a sample.
///
/// Hard bands (water / debug): [`crate::primitive::node::HydroNode::point_classification`]
/// and Durham carve / rim / apron stage cells. Terrain soft zones use
/// [`TerrainBlendStage`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionStage {
	Carve,
	Rim,
	Apron,
}

/// Soft-aware terrain band for elevation blend (includes shore / rim↔apron zones).
///
/// See [`crate::primitive::node::HydroNode::terrain_blend_classification`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainBlendStage {
	/// Wet carve: \(\phi \le 0\).
	Carve,
	/// Rim-side shore blend: \(0 < \phi \le \mu\).
	SoftShore,
	/// Pure rim bank: \(\mu < \phi < r_{\mathrm{rim}}(p) - \nu\).
	Rim,
	/// Soft rim↔apron: \(\lvert\phi - r_{\mathrm{rim}}(p)\rvert \le \nu\).
	SoftRimApron,
	/// Pure apron grade beyond the soft rim outer.
	Apron,
}

/// Target berm width outside wet support for lake / stream rims (world units).
pub const TARGET_RIM_WIDTH: f32 = 25.0;

/// Default hard cap on add-only rim height noise.
pub const DEFAULT_RIM_UPLIFT_CAP: f32 = 1.5;

/// Per-node carve / rim / apron knobs.
///
/// Two independent noisy radii (world units along occupancy \(\phi\)):
/// - **Shore** (`boundary_noise`): warps wet \(\phi = 0\) (centroid → ring).
/// - **Rim outer** (`rim_boundary_noise`): warps \(r_{\mathrm{rim}}(p)\) (ring → apron).
///
/// Classification: carve \(\phi \le 0\), rim \(0 < \phi < r_{\mathrm{rim}}(p)\),
/// apron \(r_{\mathrm{rim}}(p) \le \phi < r_{\mathrm{rim}}(p) + r_{\mathrm{apron}}\).
/// Terrain softens carve↔rim across \(\phi \in [-s, +s]\) and rim↔apron across
/// \(\phi \in [r_{\mathrm{rim}}-a, r_{\mathrm{rim}}+a]\) (water ownership stays hard
/// at \(\phi = 0\)).
#[derive(Debug, Clone)]
pub struct HydroParams {
	pub rim: RimParams,
	pub apron: ApronParams,
	/// Drawn add-only rim height noise (baked at leaf construct).
	pub rim_height: RegionNoise,
	/// Shore outline: warps wet occupancy via `φ += sample_boundary` (centroid → ring).
	pub boundary_noise: Option<RegionNoise>,
	/// Rim-outer outline: warps \(r_{\mathrm{rim}}(p) = r_{\mathrm{rim}} + sample_boundary\)
	/// (ring → apron). Independent of [`Self::boundary_noise`].
	pub rim_boundary_noise: Option<RegionNoise>,
	/// Half-width (wu) of soft terrain blend across \(\phi = 0\) (carve ↔ rim).
	///
	/// `0` restores a hard class switch. Typical leaves use a few world units so
	/// the bank meets the carved bed without a vertical cliff along \(\phi = 0\).
	pub shore_blend: f32,
	/// Half-width (wu) of soft terrain blend across the noisy rim outer
	/// (rim ↔ apron). `0` restores a hard class switch at the berm outer.
	pub rim_apron_blend: f32,
}

impl Default for HydroParams {
	fn default() -> Self {
		Self {
			rim: RimParams::default(),
			apron: ApronParams::default(),
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			boundary_noise: None,
			rim_boundary_noise: None,
			shore_blend: 4.0,
			rim_apron_blend: 4.0,
		}
	}
}

impl HydroParams {
	pub fn correction_pad(&self) -> f32 {
		// Shore / rim-outer noise amps are fractions of these bands — do not
		// stack them again on top of rim + apron.
		(self.rim.width + self.apron.width).max(0.0)
	}

	/// Peak absolute amplitude of [`Self::boundary_noise`] (0 when unset).
	pub fn boundary_noise_amp(&self) -> f32 {
		self.boundary_noise
			.as_ref()
			.map(|n| n.noise.params().amplitude.abs())
			.unwrap_or(0.0)
	}

	/// Peak absolute amplitude of [`Self::rim_boundary_noise`] (0 when unset).
	pub fn rim_boundary_noise_amp(&self) -> f32 {
		self.rim_boundary_noise
			.as_ref()
			.map(|n| n.noise.params().amplitude.abs())
			.unwrap_or(0.0)
	}

	/// Effective shore-blend half-width, clamped into the rim band.
	pub fn shore_blend_half(&self) -> f32 {
		let rim_w = self.rim.width.max(0.0);
		self.shore_blend.max(0.0).min(rim_w.max(0.0) * 0.95)
	}

	/// Effective rim↔apron blend half-width, clamped into both bands.
	pub fn rim_apron_blend_half(&self) -> f32 {
		let rim_w = self.rim.width.max(0.0);
		let apron_w = self.apron.width.max(0.0);
		let cap = (rim_w * 0.45).min(apron_w * 0.45);
		self.rim_apron_blend.max(0.0).min(cap)
	}

	/// Leaf default: a slice of rim width, at least half the boundary-indent amp.
	pub fn recommend_shore_blend(rim_w: f32, shore_amp: f32) -> f32 {
		(rim_w.max(0.0) * 0.2).clamp(2.0, 8.0).max(shore_amp.max(0.0) * 0.5)
	}

	/// Raise-only bank target at a sample given free-surface \(W\).
	pub fn bank_target(&self, water_surface: f32, p: Vec2) -> f32 {
		let base = self.rim.shelf_anchor.unwrap_or(water_surface) + self.rim.lift.max(0.0);
		let mut rim_noise = self.rim_height.sample_height(p).abs();
		rim_noise = rim_noise.min(self.rim.uplift_cap.max(0.0));
		base + rim_noise
	}
}

/// Shared rim / apron policy used when wrapping bare primitives (tests / helpers).
#[derive(Debug, Clone)]
pub struct ComplexParams {
	pub rim_lift: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub rim_height: RegionNoise,
	pub rim_uplift_cap: f32,
}

impl Default for ComplexParams {
	fn default() -> Self {
		Self {
			rim_lift: 1.1,
			rim_width: 4.0,
			apron_width: 8.0,
			rim_height: RegionNoise::from_seed(0, 0.02, 0.0),
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
		}
	}
}

impl ComplexParams {
	pub fn with_rim_noise(mut self, noise: RegionNoise, cap: f32) -> Self {
		self.rim_height = noise;
		self.rim_uplift_cap = cap.max(0.0);
		self
	}

	/// Bake into per-node [`HydroParams`] (no shelf / shore noise).
	pub fn into_params(self) -> HydroParams {
		HydroParams {
			rim: RimParams {
				width: self.rim_width,
				lift: self.rim_lift,
				shelf_anchor: None,
				uplift_cap: self.rim_uplift_cap,
				..RimParams::default()
			},
			apron: ApronParams { width: self.apron_width, ..ApronParams::default() },
			rim_height: self.rim_height,
			boundary_noise: None,
			rim_boundary_noise: None,
			shore_blend: HydroParams::default().shore_blend,
			rim_apron_blend: HydroParams::default().rim_apron_blend,
		}
	}
}
