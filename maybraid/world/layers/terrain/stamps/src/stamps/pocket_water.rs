//! Stamp Pocket Waters (small hydrology chains) — [RFC-105 §3.8.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#384-jersey-pocket-waters-small-hydrology-chains).
//!
//! Height shaping only: pond bowl, outlet lip, short run. Wet rendering is deferred.

use crate::config::{
	DownhillPair, FractalAnchors, HysteresisSpine, JitteredCenter, MidpointGrading,
	SoftmaskAlongSpine,
};
use crate::modulation::{RegionAffineModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::Bounds2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PocketTermination {
	Sink,
	MarshHint,
	HandOff,
}

#[derive(Debug, Clone, Copy)]
pub struct PocketWaterParams {
	pub termination: PocketTermination,
	pub pond_frac: f32,
	/// Pond depth (world units); modulated by [`StampStrength`].
	pub pond_depth: f32,
	pub run_width_frac: f32,
	/// Run incision depth; modulated by [`StampStrength`].
	pub run_depth: f32,
}

impl Default for PocketWaterParams {
	fn default() -> Self {
		Self {
			termination: PocketTermination::HandOff,
			pond_frac: 0.18,
			pond_depth: 10.0,
			run_width_frac: 0.09,
			run_depth: 6.0,
		}
	}
}

impl StampStrength for PocketWaterParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.pond_depth = scale_additive(self.pond_depth, strength);
		self.run_depth = scale_additive(self.run_depth, strength);
		self
	}
}

#[derive(Debug, Clone)]
pub struct PocketWater {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: PocketWaterParams,
	pub drainage_id: u32,
	pub pond_center: Vec2,
	pub run: Vec<Vec2>,
	pub stamp: StampSet,
}

impl PocketWater {
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: PocketWaterParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let short = bounds.extent().min_element().max(1.0);
		let pond_depth = params.pond_depth;
		let run_depth = params.run_depth;
		let drainage_id = seed.wrapping_mul(0x9E37_79B9);
		let pond_center = JitteredCenter::default().sample(bounds, seed, 500);
		let pond_r = short * params.pond_frac.clamp(0.1, 0.35);
		let pond_noise = RegionNoise::from_seed(seed.wrapping_add(1), 0.014, pond_r * 0.08);

		// Relative pond bowl (scale=1). Soft outer apron is the bank/rim — no lift.
		let mut modulations = vec![StampModulation::Affine(
			RegionAffineModulation::new(
				Region2D::Circle(CircleRegion { center: pond_center, radius: pond_r }),
				1.0,
				-pond_depth,
				pond_r * 0.45,
				pond_r * 1.1,
			)
			.with_noise(pond_noise.clone()),
		)];

		// Outlet geometry for the run start only. A shallow *extra* cut toward the
		// outlet (still depression-only) reads as a lip without raising terrain.
		let (_, end_anchor) = FractalAnchors::default().sample(bounds, seed, 510);
		let lip = pond_center.lerp(end_anchor, 0.35);
		modulations.push(StampModulation::Affine(
			RegionAffineModulation::new(
				Region2D::Circle(CircleRegion { center: lip, radius: pond_r * 0.4 }),
				1.0,
				-pond_depth * 0.12,
				pond_r * 0.15,
				pond_r * 0.45,
			)
			.with_noise(pond_noise),
		));

		let run = HysteresisSpine::default().build(bounds, seed.wrapping_add(21), lip, end_anchor);
		let a = *run.first().unwrap_or(&lip);
		let b = *run.last().unwrap_or(&end_anchor);
		let (s, sh, e, eh) = DownhillPair::order(a, b, height_at);
		let run_w = short * params.run_width_frac.clamp(0.05, 0.22);
		let run_noise = RegionNoise::from_seed(seed.wrapping_add(2), 0.018, run_w * 0.08);
		modulations.extend(SoftmaskAlongSpine::corridor().even_for_extent(short).build_incision(
			&run,
			run_w,
			run_depth,
			0.4,
			1.15,
			&run_noise,
			Vec2::ZERO,
		));
		modulations.push(MidpointGrading::default().build_depression(
			s,
			sh - run_depth * 0.3,
			e,
			eh - run_depth * 0.1,
			run_w * 1.35,
			run_noise,
		));

		let mut semantics = StampSemantics::default()
			.with_drainage_id(drainage_id)
			.with_tag("pocket_water")
			.with_tag("bank")
			.with_tag("littoral")
			.with_tag("reach")
			.with_tag("flow_direction")
			// Height-only stand-in for a later water-surface target.
			.with_tag("water_surface_target");
		semantics = match params.termination {
			PocketTermination::Sink => semantics.with_tag("termination_sink"),
			PocketTermination::MarshHint => semantics.with_tag("termination_marsh"),
			PocketTermination::HandOff => semantics.with_tag("termination_handoff"),
		};

		Self {
			bounds,
			seed,
			params,
			drainage_id,
			pond_center,
			run: run.clone(),
			stamp: StampSet {
				modulations,
				spine: {
					let mut spine = vec![pond_center, lip];
					spine.extend(run);
					spine
				},
				semantics,
			},
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, PocketWaterParams::default(), None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pocket_shares_drainage_id() -> anyhow::Result<()> {
		let p = PocketWater::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 320.0, 320.0), 11);
		assert_eq!(p.stamp.semantics.drainage_id, Some(p.drainage_id));
		assert!(p.stamp.modulations.len() >= 3);
		Ok(())
	}

	#[test]
	fn pocket_never_raises_terrain() -> anyhow::Result<()> {
		let p = PocketWater::from_bounds(
			Bounds2::from_xz(0.0, 0.0, 320.0, 320.0),
			11,
			PocketWaterParams::default(),
			Some(&|_, _| 40.0),
		);
		let base = 40.0;
		let probes = [
			p.pond_center,
			p.run.first().copied().unwrap_or(p.pond_center),
			p.run.get(p.run.len() / 2).copied().unwrap_or(p.pond_center),
			p.run.last().copied().unwrap_or(p.pond_center),
			p.pond_center + Vec2::new(p.params.pond_frac * 160.0, 0.0),
		];
		for q in probes {
			assert!(p.stamp.apply_elevation(base, q.x, q.y) <= base + 1e-3, "raised at {q:?}");
		}
		// Pond floor should still cut.
		assert!(p.stamp.apply_elevation(base, p.pond_center.x, p.pond_center.y) < base);
		Ok(())
	}
}
