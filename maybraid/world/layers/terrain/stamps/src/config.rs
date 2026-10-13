//! Shared construction helpers for landform stamps.
//!
//! Each helper owns the knobs that only that operation needs. Stamp families
//! compose the helpers they use; there is no single umbrella config.

use crate::modulation::{RegionAffineModulation, RegionGradingModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

/// Deterministic start/end anchors as fractions of a bound's extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FractalAnchors {
	/// Minimum start position as a fraction of extent (XZ → `Vec2` xy).
	pub start_min_frac: Vec2,
	/// Additional random span for start (added to [`Self::start_min_frac`]).
	pub start_span_frac: Vec2,
	/// Minimum end position as a fraction of extent.
	pub end_min_frac: Vec2,
	/// Additional random span for end.
	pub end_span_frac: Vec2,
}

impl Default for FractalAnchors {
	fn default() -> Self {
		Self {
			start_min_frac: Vec2::new(0.12, 0.18),
			start_span_frac: Vec2::new(0.28, 0.30),
			end_min_frac: Vec2::new(0.55, 0.50),
			end_span_frac: Vec2::new(0.30, 0.32),
		}
	}
}

impl FractalAnchors {
	pub fn sample(&self, bounds: Bounds2, seed: u32, salt: u32) -> (Vec2, Vec2) {
		let hash = SeededHash::new(seed.wrapping_add(salt));
		let extent = bounds.extent();
		let start = bounds.project(Vec2::new(
			bounds.min.x
				+ (self.start_min_frac.x + self.start_span_frac.x * hash.unit(1)) * extent.x,
			bounds.min.y
				+ (self.start_min_frac.y + self.start_span_frac.y * hash.unit(2)) * extent.y,
		));
		let end = bounds.project(Vec2::new(
			bounds.min.x + (self.end_min_frac.x + self.end_span_frac.x * hash.unit(3)) * extent.x,
			bounds.min.y + (self.end_min_frac.y + self.end_span_frac.y * hash.unit(4)) * extent.y,
		));
		(start, end)
	}
}

/// Jittered placement of a single point inside a bound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitteredCenter {
	/// Minimum center position as a fraction of extent.
	pub min_frac: Vec2,
	/// Random span for center (added to [`Self::min_frac`]).
	pub span_frac: Vec2,
}

impl Default for JitteredCenter {
	fn default() -> Self {
		Self { min_frac: Vec2::splat(0.25), span_frac: Vec2::splat(0.5) }
	}
}

impl JitteredCenter {
	pub fn sample(&self, bounds: Bounds2, seed: u32, salt: u32) -> Vec2 {
		let hash = SeededHash::new(seed.wrapping_add(salt));
		let extent = bounds.extent();
		bounds.project(Vec2::new(
			bounds.min.x + (self.min_frac.x + self.span_frac.x * hash.unit(1)) * extent.x,
			bounds.min.y + (self.min_frac.y + self.span_frac.y * hash.unit(2)) * extent.y,
		))
	}
}

/// Degree-1 hysteresis polyline from start toward end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisSpine {
	/// Path step length as a fraction of the shorter bound edge.
	pub step_frac: f32,
	/// Clamp for [`Self::step_frac`] × short edge (world units).
	pub step_min: f32,
	pub step_max: f32,
	/// Max segments in the walk.
	pub max_segments: usize,
	/// Snap-to-end radius as a multiple of step length.
	pub snap_step_mul: f32,
	/// Connect-to-end radius as a multiple of step length.
	pub connect_step_mul: f32,
}

impl Default for HysteresisSpine {
	fn default() -> Self {
		Self {
			step_frac: 0.08,
			step_min: 8.0,
			step_max: 40.0,
			max_segments: 28,
			snap_step_mul: 0.75,
			connect_step_mul: 1.6,
		}
	}
}

impl HysteresisSpine {
	pub fn walk_config(&self, bounds: Bounds2) -> HysteresisConfig {
		let short = bounds.extent().min_element().max(1.0);
		let step = (short * self.step_frac).clamp(self.step_min, self.step_max);
		HysteresisConfig {
			max_segments: self.max_segments,
			step_len: step,
			snap_radius: step * self.snap_step_mul,
			connect_radius: step * self.connect_step_mul,
			..HysteresisConfig::default()
		}
	}

	pub fn build(&self, bounds: Bounds2, seed: u32, start: Vec2, end: Vec2) -> Vec<Vec2> {
		HysteresisGraph::degree1(bounds, seed, start, end, &self.walk_config(bounds))
			.primary_polyline()
	}
}

/// Circle softmasks sampled along a polyline spine.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftmaskAlongSpine {
	/// Target samples ≈ `len / stride_divisor` (then clamped).
	/// Ignored when [`Self::spacing_half_width_frac`] is set.
	pub stride_divisor: usize,
	pub stride_min: usize,
	pub stride_max: usize,
	/// Depth/lift falls off by this fraction from head to tail of the spine.
	pub longitudinal_falloff: f32,
	/// When set, densify the path and place one sample every
	/// `frac * half_width` world units (connected corridor without sample-time
	/// polyline SDF).
	pub spacing_half_width_frac: Option<f32>,
	/// Multiplies circle radius (wider apron / more overlap along the path).
	pub radius_scale: f32,
	/// Hard cap on emitted softmask samples after densify.
	pub max_samples: usize,
}

impl Default for SoftmaskAlongSpine {
	fn default() -> Self {
		Self {
			stride_divisor: 6,
			stride_min: 1,
			stride_max: 4,
			longitudinal_falloff: 0.2,
			spacing_half_width_frac: None,
			radius_scale: 1.0,
			max_samples: 48,
		}
	}
}

impl SoftmaskAlongSpine {
	/// Connected incision corridor: densified nodes + wider overlapping circles.
	pub fn corridor() -> Self {
		Self {
			stride_divisor: 4,
			stride_min: 1,
			stride_max: 1,
			longitudinal_falloff: 0.25,
			// Spacing < diameter so neighboring full-strength cores overlap.
			spacing_half_width_frac: Some(0.55),
			radius_scale: 1.35,
			max_samples: 40,
		}
	}

	/// Gentler head→tail falloff and denser samples on larger leaves.
	///
	/// Keeps regional (high-pass) spines more evenly graded instead of collapsing
	/// to a short crest on a multi‑kilometre path.
	pub fn even_for_extent(mut self, short_edge: f32) -> Self {
		let t = (short_edge / crate::stamp::SOFTMASK_REFERENCE_SHORT).clamp(0.25, 8.0);
		let soften = t.sqrt();
		self.longitudinal_falloff = (self.longitudinal_falloff / soften).clamp(0.04, 0.5);
		let samples = (self.max_samples as f32 * soften).round() as usize;
		self.max_samples = samples.clamp(8, 96);
		self
	}

	/// Depression when `offset` is negative; lift when positive.
	pub fn build(
		&self,
		path: &[Vec2],
		half_width: f32,
		scale: f32,
		offset: f32,
		inner_frac: f32,
		outer_frac: f32,
		noise: &RegionNoise,
		lateral: Vec2,
	) -> Vec<StampModulation> {
		let mut out = Vec::new();
		if path.is_empty() {
			return out;
		}
		let radius = (half_width * self.radius_scale).max(1.0);
		let samples = match self.spacing_half_width_frac {
			Some(frac) => densify_polyline(path, (half_width * frac).max(1.0), self.max_samples),
			None => {
				let stride = ((path.len() / self.stride_divisor.max(1)).max(self.stride_min))
					.min(self.stride_max);
				path.iter().step_by(stride).copied().take(self.max_samples.max(1)).collect()
			}
		};
		let n = samples.len().saturating_sub(1).max(1) as f32;
		let inner_r = radius * inner_frac;
		let outer_r = radius * outer_frac;
		for (i, p) in samples.iter().enumerate() {
			let t = i as f32 / n;
			let local = offset * (1.0 - self.longitudinal_falloff * t);
			let region = Region2D::Circle(CircleRegion { center: *p + lateral, radius });
			out.push(StampModulation::Affine(
				RegionAffineModulation::new(region, scale, local, inner_r, outer_r)
					.with_noise(noise.clone()),
			));
		}
		out
	}

	/// Relative incision: keep base relief (`scale = 1`) and apply a negative offset.
	pub fn build_incision(
		&self,
		path: &[Vec2],
		half_width: f32,
		depth: f32,
		inner_frac: f32,
		outer_frac: f32,
		noise: &RegionNoise,
		lateral: Vec2,
	) -> Vec<StampModulation> {
		self.build(path, half_width, 1.0, -depth.abs(), inner_frac, outer_frac, noise, lateral)
	}
}

/// Insert vertices so consecutive points are at most `max_spacing` apart.
///
/// Build-time only — evaluation still uses plain circle softmasks.
fn densify_polyline(path: &[Vec2], max_spacing: f32, max_samples: usize) -> Vec<Vec2> {
	let max_samples = max_samples.max(2);
	if path.is_empty() {
		return Vec::new();
	}
	if path.len() == 1 {
		return vec![path[0]];
	}
	let max_spacing = max_spacing.max(1.0);
	let mut out = Vec::with_capacity(path.len() * 2);
	out.push(path[0]);
	for window in path.windows(2) {
		if out.len() >= max_samples {
			break;
		}
		let a = window[0];
		let b = window[1];
		let dist = a.distance(b);
		if dist <= max_spacing {
			if out.last().map(|p| p.distance(b) > 1e-3).unwrap_or(true) {
				out.push(b);
			}
			continue;
		}
		let steps = (dist / max_spacing).ceil() as usize;
		for i in 1..=steps {
			if out.len() >= max_samples {
				break;
			}
			let t = i as f32 / steps as f32;
			let p = a.lerp(b, t);
			if out.last().map(|q| q.distance(p) > 1e-3).unwrap_or(true) {
				out.push(p);
			}
		}
	}
	let end = path[path.len() - 1];
	if out.len() < max_samples && out.last().map(|p| p.distance(end) > 1e-3).unwrap_or(true) {
		out.push(end);
	}
	out
}

#[cfg(test)]
mod densify_tests {
	use super::*;

	#[test]
	fn densify_fills_long_segments() -> anyhow::Result<()> {
		let path = vec![Vec2::ZERO, Vec2::new(100.0, 0.0)];
		let dense = densify_polyline(&path, 25.0, 40);
		assert!(dense.len() >= 5);
		for w in dense.windows(2) {
			assert!(w[0].distance(w[1]) <= 25.0 + 1e-3);
		}
		Ok(())
	}

	#[test]
	fn corridor_emits_overlapping_samples() -> anyhow::Result<()> {
		let path = vec![Vec2::ZERO, Vec2::new(200.0, 0.0)];
		let noise = RegionNoise::from_seed(1, 0.02, 1.0);
		let mods = SoftmaskAlongSpine::corridor().build_incision(
			&path,
			20.0,
			10.0,
			0.4,
			1.15,
			&noise,
			Vec2::ZERO,
		);
		// Sparse 2-point path should densify into many overlapping softmasks.
		assert!(mods.len() >= 8);
		Ok(())
	}
}

/// Single grading region centered between two endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidpointGrading {
	/// Grading region radius as a multiple of corridor half-width.
	pub radius_half_width_mul: f32,
	/// Softmask inner radius as a fraction of corridor half-width.
	pub inner_half_width_frac: f32,
	/// Softmask outer radius as a fraction of corridor half-width.
	pub outer_half_width_frac: f32,
}

impl Default for MidpointGrading {
	fn default() -> Self {
		Self { radius_half_width_mul: 1.5, inner_half_width_frac: 0.4, outer_half_width_frac: 1.05 }
	}
}

impl MidpointGrading {
	pub fn build(
		&self,
		start: Vec2,
		start_h: f32,
		end: Vec2,
		end_h: f32,
		half_width: f32,
		noise: RegionNoise,
	) -> StampModulation {
		self.build_inner(start, start_h, end, end_h, half_width, noise, false)
	}

	/// Downhill floor bias that never raises above the incoming surface.
	pub fn build_depression(
		&self,
		start: Vec2,
		start_h: f32,
		end: Vec2,
		end_h: f32,
		half_width: f32,
		noise: RegionNoise,
	) -> StampModulation {
		self.build_inner(start, start_h, end, end_h, half_width, noise, true)
	}

	fn build_inner(
		&self,
		start: Vec2,
		start_h: f32,
		end: Vec2,
		end_h: f32,
		half_width: f32,
		noise: RegionNoise,
		depression_only: bool,
	) -> StampModulation {
		let center = (start + end) * 0.5;
		let region = Region2D::Circle(CircleRegion {
			center,
			radius: half_width * self.radius_half_width_mul,
		});
		let mut grading = RegionGradingModulation::new(
			region,
			start,
			start_h,
			end,
			end_h,
			half_width * self.inner_half_width_frac,
			half_width * self.outer_half_width_frac,
		)
		.with_noise(noise);
		if depression_only {
			grading = grading.depression_only();
		}
		StampModulation::Grading(grading)
	}
}

/// Order endpoints so grade runs downhill when heights are available.
///
/// Stateless utility (no tunables).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DownhillPair;

impl DownhillPair {
	pub fn order(
		a: Vec2,
		b: Vec2,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> (Vec2, f32, Vec2, f32) {
		let h0 = height_at.map(|f| f(a.x, a.y)).unwrap_or(0.0);
		let h1 = height_at.map(|f| f(b.x, b.y)).unwrap_or(0.0);
		if h0 >= h1 {
			(a, h0, b, h1)
		} else {
			(b, h1, a, h0)
		}
	}
}
