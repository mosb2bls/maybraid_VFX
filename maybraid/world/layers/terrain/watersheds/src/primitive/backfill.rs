//! Per-node post-carve terrain height noise (basin / rim / node).
//!
//! Applied **after** carve → rim → apron elevation blend so hummocks / shore
//! grit can rise into an already-corrected bed. One optional
//! [`HydroBackfill`] lives on each [`crate::primitive::node::HydroNode`].
//! Multi-node terrain blends soft-max each node's raise over bare \(h_0\)
//! (see [`crate::primitive::node::HydroNode::elevation_blend`]) so corridor
//! overlaps do not sum grit.
//!
//! Amplitude for basin recipes is **depth-incentive**: callers supply a
//! freeboard (depth below \(W\)) and a [`BasinBackfillParams::depth_frac`].

use crate::primitive::node::HydroNode;
use bevy_math::Vec2;
use procedural_common::Smoothstep01;
use terrain_stamps::RegionNoise;

/// Which footprint a backfill weights within (along occupancy \(\phi\)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HydroBackfillKind {
	/// Inside the wet core (\(\phi \le 0\)), soft fade past shore.
	Basin,
	/// Band centered on wet shore \(\phi = 0\) (either side into carve and rim).
	Rim,
	/// Full node correction support (rim + apron).
	Node,
}

/// Runtime backfill recipe attached to a single [`HydroNode`].
#[derive(Debug, Clone)]
pub enum HydroBackfill {
	Basin(BasinBackfill),
	Rim(RimBackfill),
	Node(NodeBackfill),
}

impl HydroBackfill {
	pub fn kind(&self) -> HydroBackfillKind {
		match self {
			Self::Basin(_) => HydroBackfillKind::Basin,
			Self::Rim(_) => HydroBackfillKind::Rim,
			Self::Node(_) => HydroBackfillKind::Node,
		}
	}

	/// Softmask weight in \([0, 1]\) at `p` for this node's occupancy.
	pub fn weight(&self, node: &HydroNode, p: Vec2) -> f32 {
		match self {
			Self::Basin(b) => b.weight(node, p),
			Self::Rim(b) => b.weight(node, p),
			Self::Node(b) => b.weight(node, p),
		}
	}

	/// Height delta before weight (raise-only when configured).
	pub fn delta(&self, _node: &HydroNode, p: Vec2) -> f32 {
		match self {
			Self::Basin(b) => b.delta(p),
			Self::Rim(b) => b.delta(p),
			Self::Node(b) => b.delta(p),
		}
	}

	/// `h + weight * delta`.
	pub fn compose(&self, h: f32, node: &HydroNode, p: Vec2) -> f32 {
		let w = self.weight(node, p);
		if w <= 1e-6 {
			return h;
		}
		h + w * self.delta(node, p)
	}
}

/// Basin: soft inside wet carve, fade across \(\phi \in [0, fade]\).
#[derive(Debug, Clone)]
pub struct BasinBackfill {
	pub noise: RegionNoise,
	pub fade: f32,
	pub add_only: bool,
}

impl BasinBackfill {
	pub fn weight(&self, node: &HydroNode, p: Vec2) -> f32 {
		let phi = node.phi(p);
		let fade = self.fade.max(1e-3);
		if phi <= 0.0 {
			1.0
		} else if phi >= fade {
			0.0
		} else {
			let t = phi / fade;
			1.0 - Smoothstep01::eval(t)
		}
	}

	pub fn delta(&self, p: Vec2) -> f32 {
		sample_delta(&self.noise, p, self.add_only)
	}
}

/// Rim: peak at \(\phi = 0\), half-width [`Self::band`] either side (symmetric).
#[derive(Debug, Clone)]
pub struct RimBackfill {
	pub noise: RegionNoise,
	/// Half-width (wu) of the effect about the wet shore (into carve and rim).
	pub band: f32,
	pub add_only: bool,
}

impl RimBackfill {
	pub fn weight(&self, node: &HydroNode, p: Vec2) -> f32 {
		let band = self.band.max(1e-3);
		let phi = node.phi(p);
		let t = (phi.abs() / band).clamp(0.0, 1.0);
		1.0 - Smoothstep01::eval(t)
	}

	pub fn delta(&self, p: Vec2) -> f32 {
		sample_delta(&self.noise, p, self.add_only)
	}
}

/// Node: soft across rim + apron support.
#[derive(Debug, Clone)]
pub struct NodeBackfill {
	pub noise: RegionNoise,
	pub fade: f32,
	pub add_only: bool,
}

impl NodeBackfill {
	pub fn weight(&self, node: &HydroNode, p: Vec2) -> f32 {
		let phi = node.phi(p);
		let support = node.params.rim.width.max(0.0) + node.params.apron.width.max(0.0);
		let fade = self.fade.max(1e-3);
		let outer = support + fade;
		if phi <= support {
			1.0
		} else if phi >= outer {
			0.0
		} else {
			let t = (phi - support) / fade;
			1.0 - Smoothstep01::eval(t)
		}
	}

	pub fn delta(&self, p: Vec2) -> f32 {
		sample_delta(&self.noise, p, self.add_only)
	}
}

#[inline]
fn sample_delta(noise: &RegionNoise, p: Vec2, add_only: bool) -> f32 {
	let raw = noise.sample_height(p);
	if add_only {
		raw.abs()
	} else {
		raw
	}
}

/// Depth-incentive authoring for a basin backfill noise draw.
///
/// World amplitude is `freeboard * depth_frac` via [`Self::amp_for_freeboard`].
#[derive(Debug, Clone, Copy)]
pub struct BasinBackfillParams {
	/// Noise amplitude as a multiple of bowl freeboard (`1` ≈ full-scale raise
	/// equals depth below \(W\)).
	pub depth_frac: f32,
	pub freq: f32,
	pub fade: f32,
	/// FBM octave count (≥1). Extra octaves densify mound packing.
	pub octaves: u8,
	/// Raise-only mounds (no bipolar digs into the carved bed).
	pub add_only: bool,
}

impl Default for BasinBackfillParams {
	fn default() -> Self {
		Self { depth_frac: 1.0, freq: 0.04, fade: 2.0, octaves: 1, add_only: false }
	}
}

impl BasinBackfillParams {
	/// `freeboard * depth_frac` (both non-negative).
	pub fn amp_for_freeboard(&self, freeboard: f32) -> f32 {
		freeboard.max(0.0) * self.depth_frac.max(0.0)
	}

	/// Near-surface peaking: depth_frac large enough that unit noise crests at \(W\).
	pub fn near_surface(mut self, freeboard: f32, peak_above_w: f32, crest_unit: f32) -> Self {
		let fb = freeboard.max(1e-3);
		let u = crest_unit.clamp(0.05, 1.0);
		let amp = (fb / u).max(fb + peak_above_w.max(0.0));
		self.depth_frac = amp / fb;
		self.add_only = true;
		self
	}

	/// Milder fill: smaller fraction of freeboard (farther from surface on average).
	pub fn farther_from_surface(mut self, depth_frac: f32) -> Self {
		self.depth_frac = depth_frac.max(0.0);
		self
	}

	/// Build a [`HydroBackfill::Basin`] with amplitude from `freeboard`.
	pub fn sample_over_freeboard(
		&self,
		freeboard: f32,
		seed: u32,
		salt_offset: u32,
	) -> HydroBackfill {
		use procedural_common::{NoiseParams, NoiseType};

		let noise = RegionNoise::from_params(NoiseParams {
			seed: seed.wrapping_add(salt_offset) as i32,
			frequency: self.freq.max(1.0e-4).clamp(1.0e-4, 0.14),
			amplitude: self.amp_for_freeboard(freeboard),
			octaves: self.octaves.max(1) as u32,
			noise_type: NoiseType::Perlin,
		});
		HydroBackfill::Basin(BasinBackfill {
			noise,
			fade: self.fade.max(0.25),
			add_only: self.add_only,
		})
	}
}

/// Authoring knobs for rim (shore-band) backfill.
#[derive(Debug, Clone, Copy)]
pub struct RimBackfillParams {
	/// Half-width (wu) either side of \(\phi = 0\).
	pub band: f32,
	/// World-space noise amplitude.
	pub amp: f32,
	pub freq: f32,
	pub octaves: u8,
	pub add_only: bool,
}

impl Default for RimBackfillParams {
	fn default() -> Self {
		Self { band: 14.0, amp: 3.5, freq: 0.045, octaves: 2, add_only: true }
	}
}

impl RimBackfillParams {
	/// Size band (and a matching amp) from a characteristic leaf extent.
	///
	/// `band = extent * band_frac`; amp scales with band so grit stays visible.
	/// Leaf stamps choose `band_frac` / freq in their authored modules.
	pub fn from_extent(extent: f32, band_frac: f32) -> Self {
		let e = extent.max(1.0);
		let frac = band_frac.clamp(0.05, 0.95);
		let band = (e * frac).max(4.0);
		// ~55% of band height, floored so small leaves still punch.
		let amp = (band * 0.55).clamp(5.0, 18.0);
		Self { band, amp, ..Self::default() }
	}

	/// Build a [`HydroBackfill::Rim`].
	pub fn sample(&self, seed: u32, salt_offset: u32) -> HydroBackfill {
		use procedural_common::{NoiseParams, NoiseType};

		let noise = RegionNoise::from_params(NoiseParams {
			seed: seed.wrapping_add(salt_offset) as i32,
			frequency: self.freq.max(1.0e-4).clamp(1.0e-4, 0.14),
			amplitude: self.amp.max(0.0),
			octaves: self.octaves.max(1) as u32,
			noise_type: NoiseType::Perlin,
		});
		HydroBackfill::Rim(RimBackfill { noise, band: self.band.max(0.5), add_only: self.add_only })
	}
}

/// Authoring knobs for whole-node backfill.
#[derive(Debug, Clone, Copy)]
pub struct NodeBackfillParams {
	pub amp: f32,
	pub freq: f32,
	pub fade: f32,
	pub octaves: u8,
	pub add_only: bool,
}

impl Default for NodeBackfillParams {
	fn default() -> Self {
		Self { amp: 1.5, freq: 0.03, fade: 3.0, octaves: 1, add_only: true }
	}
}

impl NodeBackfillParams {
	/// Build a [`HydroBackfill::Node`].
	pub fn sample(&self, seed: u32, salt_offset: u32) -> HydroBackfill {
		use procedural_common::{NoiseParams, NoiseType};

		let noise = RegionNoise::from_params(NoiseParams {
			seed: seed.wrapping_add(salt_offset) as i32,
			frequency: self.freq.max(1.0e-4).clamp(1.0e-4, 0.14),
			amplitude: self.amp.max(0.0),
			octaves: self.octaves.max(1) as u32,
			noise_type: NoiseType::Perlin,
		});
		HydroBackfill::Node(NodeBackfill {
			noise,
			fade: self.fade.max(0.25),
			add_only: self.add_only,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::primitive::hydro::HydroPrimitive;
	use crate::primitive::hydro::{HydroElevation, HydroFootprint, ReachProfile, ReachSegment};
	use crate::primitive::parameters::HydroParams;

	fn reach_node(half_width: f32) -> HydroNode {
		let mut params = HydroParams::default();
		params.rim.width = 4.0;
		params.apron.width = 8.0;
		HydroNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Reach(ReachSegment {
					a: Vec2::new(0.0, 0.0),
					b: Vec2::new(40.0, 0.0),
					half_width,
				}),
				elevation: HydroElevation::Reach(ReachProfile {
					surface_a: 30.0,
					surface_b: 30.0,
					center_depth: 3.0,
				}),
				influence_pad: 12.0,
			},
			params,
			12.0,
		)
	}

	#[test]
	fn basin_weight_inside_wet_outside_fade() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		let bf = BasinBackfill {
			noise: RegionNoise::from_seed(1, 0.05, 2.0),
			fade: 2.0,
			add_only: true,
		};
		let w_in = bf.weight(&node, Vec2::new(20.0, 0.0));
		let w_far = bf.weight(&node, Vec2::new(20.0, 20.0));
		anyhow::ensure!(w_in > 0.95, "wet interior weight {w_in}");
		anyhow::ensure!(w_far < 0.05, "far weight {w_far}");
		Ok(())
	}

	#[test]
	fn rim_weight_peaks_at_shore() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		let bf =
			RimBackfill { noise: RegionNoise::from_seed(2, 0.05, 2.0), band: 4.0, add_only: true };
		let w_shore = bf.weight(&node, Vec2::new(20.0, 8.0));
		let w_far = bf.weight(&node, Vec2::new(20.0, 20.0));
		anyhow::ensure!(w_shore > 0.9, "shore weight {w_shore}");
		anyhow::ensure!(w_far < 0.05, "far weight {w_far}");
		Ok(())
	}

	#[test]
	fn node_weight_covers_apron_support() -> anyhow::Result<()> {
		let node = reach_node(8.0);
		let bf =
			NodeBackfill { noise: RegionNoise::from_seed(3, 0.05, 2.0), fade: 2.0, add_only: true };
		// rim 4 + apron 8 → support 12; φ≈10 at y=18.
		let w_mid = bf.weight(&node, Vec2::new(20.0, 14.0));
		let w_far = bf.weight(&node, Vec2::new(20.0, 40.0));
		anyhow::ensure!(w_mid > 0.9, "support weight {w_mid}");
		anyhow::ensure!(w_far < 0.05, "far weight {w_far}");
		Ok(())
	}

	#[test]
	fn amp_scales_with_freeboard_and_depth_frac() -> anyhow::Result<()> {
		let p = BasinBackfillParams { depth_frac: 1.25, ..BasinBackfillParams::default() };
		anyhow::ensure!((p.amp_for_freeboard(8.0) - 10.0).abs() < 1e-4);
		anyhow::ensure!((p.amp_for_freeboard(4.0) - 5.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn rim_params_scale_from_extent() -> anyhow::Result<()> {
		let p = RimBackfillParams::from_extent(80.0, 0.45);
		anyhow::ensure!((p.band - 36.0).abs() < 1e-3, "band={}", p.band);
		anyhow::ensure!(p.amp >= 12.0, "amp should scale with band, got {}", p.amp);
		Ok(())
	}
}
