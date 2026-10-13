//! Stamp Rugged Massifs (unchained) — [RFC-105 §3.8.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#383-jersey-rugged-massifs-unchained).

use crate::config::{FractalAnchors, HysteresisSpine, SoftmaskAlongSpine};
use crate::region::RegionNoise;
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MassifStyle {
	Ridged,
	Serrated,
	CliffBanded,
}

#[derive(Debug, Clone, Copy)]
pub struct RuggedMassifParams {
	pub style: MassifStyle,
	pub width_frac: f32,
	/// Crest raise (world units); modulated by [`StampStrength`].
	pub lift: f32,
	pub crest_scale: f32,
	/// Relative elevation factor (`1.0` ≈ defaults); scales lobe projection size.
	pub strength: f32,
}

impl Default for RuggedMassifParams {
	fn default() -> Self {
		Self {
			style: MassifStyle::Ridged,
			width_frac: 0.14,
			lift: 22.0,
			crest_scale: 1.15,
			strength: 1.0,
		}
	}
}

impl StampStrength for RuggedMassifParams {
	fn with_strength(mut self, strength: f32) -> Self {
		let s = strength.max(0.0);
		self.strength = s;
		self.lift = scale_additive(self.lift, s);
		self
	}
}

#[derive(Debug, Clone)]
pub struct RuggedMassif {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: RuggedMassifParams,
	pub path: Vec<Vec2>,
	pub stamp: StampSet,
}

impl RuggedMassif {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: RuggedMassifParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let (start, end) = FractalAnchors::default().sample(bounds, seed, 100);
		let path = HysteresisSpine::default().build(bounds, seed.wrapping_add(19), start, end);
		let half_width = short * params.width_frac.clamp(0.06, 0.3);
		let (inner, outer, width_mul) = match params.style {
			MassifStyle::Ridged => (0.4, 0.65, 1.4),
			MassifStyle::Serrated => (0.3, 0.55, 1.4),
			MassifStyle::CliffBanded => (0.35, 0.7, 1.2),
		};

		// Elevation strength grows softmask circle size (`radius_scale`) and thins
		// path samples (`stride_max` / `max_samples`) so overlap stays similar —
		// growing radius alone stacks a*e+b and spikes height + roughness.
		let lobe_scale = params.strength.max(0.25);
		let spine = SoftmaskAlongSpine {
			stride_divisor: 6,
			stride_min: 1,
			stride_max: ((4.0 * lobe_scale).round() as usize).clamp(2, 24),
			longitudinal_falloff: 0.2,
			spacing_half_width_frac: None,
			radius_scale: lobe_scale.clamp(0.5, 4.0),
			max_samples: ((48.0 / lobe_scale).round() as usize).clamp(6, 48),
		};
		let lobe_r = half_width * width_mul * spine.radius_scale;
		let noise = RegionNoise::from_seed(
			seed.wrapping_add(8),
			(2.0 / lobe_r.max(1.0)).clamp(0.002, 0.04),
			(lobe_r * 0.04).min(10.0),
		);

		let mut modulations = spine.build(
			&path,
			half_width * width_mul,
			params.crest_scale,
			params.lift,
			inner,
			outer,
			&noise,
			Vec2::ZERO,
		);
		if matches!(params.style, MassifStyle::Serrated | MassifStyle::CliffBanded) {
			let axis = (*path.last().unwrap_or(&end) - *path.first().unwrap_or(&start))
				.normalize_or_zero();
			let perp = Vec2::new(-axis.y, axis.x);
			let side = if hash.unit(12) > 0.5 { 1.0 } else { -1.0 };
			modulations.extend(spine.build(
				&path,
				half_width * 0.55,
				params.crest_scale * 0.9,
				params.lift * 0.7,
				inner,
				outer,
				&noise,
				perp * half_width * spine.radius_scale * 0.6 * side,
			));
		}

		let mut semantics = StampSemantics::default()
			.with_tag("massif")
			.with_tag("exposure")
			.with_tag("rockiness");
		semantics = match params.style {
			MassifStyle::Ridged => semantics.with_tag("ridged"),
			MassifStyle::Serrated => semantics.with_tag("serrated"),
			MassifStyle::CliffBanded => semantics.with_tag("cliff_banded"),
		};

		Self {
			bounds,
			seed,
			params,
			path: path.clone(),
			stamp: StampSet { modulations, spine: path, semantics },
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, RuggedMassifParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn massif_raises_crest() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let m = RuggedMassif::from_bounds_default(bounds, 3);
		let p = m.path[m.path.len() / 2];
		let h_crest = m.stamp.apply_elevation(40.0, p.x, p.y);
		let h_out = m.stamp.apply_elevation(40.0, bounds.min.x + 2.0, bounds.min.y + 2.0);
		assert!(h_crest > h_out);
		Ok(())
	}

	#[test]
	fn massif_strength_scales_lift() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let weak =
			RuggedMassif::from_bounds(bounds, 3, RuggedMassifParams::default().with_strength(0.5));
		let strong =
			RuggedMassif::from_bounds(bounds, 3, RuggedMassifParams::default().with_strength(2.0));
		let p = weak.path[weak.path.len() / 2];
		let dw = weak.stamp.apply_elevation(40.0, p.x, p.y) - 40.0;
		let ds = strong.stamp.apply_elevation(40.0, p.x, p.y) - 40.0;
		assert!(ds > dw * 1.5, "strong={ds} weak={dw}");
		Ok(())
	}

	#[test]
	fn stronger_massif_emits_fewer_softmask_lobes() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 1600.0, 1600.0);
		let weak =
			RuggedMassif::from_bounds(bounds, 3, RuggedMassifParams::default().with_strength(0.5));
		let strong =
			RuggedMassif::from_bounds(bounds, 3, RuggedMassifParams::default().with_strength(3.0));
		assert!(
			strong.stamp.modulations.len() < weak.stamp.modulations.len(),
			"strong={} weak={}",
			strong.stamp.modulations.len(),
			weak.stamp.modulations.len()
		);
		Ok(())
	}
}
