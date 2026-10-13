//! Stamp Canyons (confined incision) — [RFC-105 §3.8.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#387-jersey-canyons-confined-incision).

use crate::config::{
	DownhillPair, FractalAnchors, HysteresisSpine, MidpointGrading, SoftmaskAlongSpine,
};
use crate::region::RegionNoise;
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::Bounds2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanyonVariant {
	/// Single enclosed reach of incision.
	Unchained,
	/// Upper slot → wider box → exit ramp along one centerline.
	Chained,
}

#[derive(Debug, Clone, Copy)]
pub struct CanyonParams {
	pub variant: CanyonVariant,
	pub width_frac: f32,
	/// Incision depth (world units); modulated by [`StampStrength`].
	pub depth: f32,
	pub confinement: f32,
}

impl Default for CanyonParams {
	fn default() -> Self {
		Self { variant: CanyonVariant::Unchained, width_frac: 0.12, depth: 28.0, confinement: 0.8 }
	}
}

impl StampStrength for CanyonParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.depth = scale_additive(self.depth, strength);
		self
	}
}

#[derive(Debug, Clone)]
pub struct Canyon {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: CanyonParams,
	pub path: Vec<Vec2>,
	pub stamp: StampSet,
}

impl Canyon {
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: CanyonParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let short = bounds.extent().min_element().max(1.0);
		let depth = params.depth;
		let (start, end) = FractalAnchors::default().sample(bounds, seed, 300);
		let path = HysteresisSpine::default().build(bounds, seed.wrapping_add(31), start, end);
		let a = *path.first().unwrap_or(&start);
		let b = *path.last().unwrap_or(&end);
		let (start_pt, start_h, end_pt, end_h) = DownhillPair::order(a, b, height_at);

		let base_w =
			short * params.width_frac.clamp(0.05, 0.28) * params.confinement.clamp(0.4, 1.2);
		let noise = RegionNoise::from_seed(seed.wrapping_add(4), 0.02, base_w * 0.08);
		// Densified overlapping circles (build-time); no polyline SDF at sample time.
		let spine = SoftmaskAlongSpine::corridor().even_for_extent(short);

		// Relative incision along the path (scale=1, negative offset). No absolute floor.
		// Soft outer apron keeps depth connected between densified nodes.
		let mut modulations = Vec::new();
		match params.variant {
			CanyonVariant::Unchained => {
				modulations.extend(spine.build_incision(
					&path,
					base_w,
					depth,
					0.4,
					1.15,
					&noise,
					Vec2::ZERO,
				));
			}
			CanyonVariant::Chained => {
				let n = path.len().max(3);
				let s0 = 0;
				let s1 = n / 3;
				let s2 = (2 * n) / 3;
				let segments = [
					(&path[s0..=s1.min(n - 1)], base_w * 0.7, depth * 1.2),
					(&path[s1.min(n - 1)..=s2.min(n - 1)], base_w * 1.15, depth),
					(&path[s2.min(n - 1)..], base_w * 1.35, depth * 0.65),
				];
				for (seg, w, d) in segments {
					modulations.extend(spine.build_incision(
						seg,
						w,
						d,
						0.4,
						1.15,
						&noise,
						Vec2::ZERO,
					));
				}
			}
		}
		// Mild downhill bias only — never raise natural lows toward baked floors.
		modulations.push(MidpointGrading::default().build_depression(
			start_pt,
			start_h - depth * 0.4,
			end_pt,
			end_h - depth * 0.15,
			base_w * 1.4,
			noise,
		));

		let mut semantics = StampSemantics::default()
			.with_tag("canyon")
			.with_tag("wall")
			.with_tag("cliff")
			.with_tag("floor")
			.with_tag("thalweg");
		semantics = match params.variant {
			CanyonVariant::Unchained => semantics.with_tag("unchained"),
			CanyonVariant::Chained => semantics
				.with_tag("chained")
				.with_tag("slot")
				.with_tag("box_canyon")
				.with_tag("exit_ramp"),
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
		Self::from_bounds(bounds, seed, CanyonParams::default(), None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn canyon_cuts_floor() -> anyhow::Result<()> {
		let c = Canyon::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 400.0, 400.0), 5);
		let p = c.path[c.path.len() / 2];
		let h = c.stamp.apply_elevation(80.0, p.x, p.y);
		assert!(h < 80.0);
		assert!(c.stamp.semantics.tags.contains(&"canyon"));
		Ok(())
	}

	#[test]
	fn canyon_does_not_raise_natural_valley() -> anyhow::Result<()> {
		let c = Canyon::from_bounds(
			Bounds2::from_xz(0.0, 0.0, 400.0, 400.0),
			5,
			CanyonParams::default(),
			Some(&|_, _| 60.0),
		);
		let p = c.path[c.path.len() / 2];
		// Endpoint grade sits near ~40–50; a natural low must not be lifted toward it.
		assert!(c.stamp.apply_elevation(10.0, p.x, p.y) <= 10.0);
		Ok(())
	}
}
