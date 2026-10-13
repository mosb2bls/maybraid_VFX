//! Stamp Valley Trains (chained valleys) — [RFC-105 §3.8.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#386-jersey-valley-trains-chained-valleys).

use crate::config::{FractalAnchors, HysteresisSpine};
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use crate::stamps::valley_basin::{
	ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind,
};
use bevy_math::Vec2;
use procedural_common::Bounds2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValleyTrainSegmentRole {
	UpperGorge,
	MiddleGlide,
	LowerFloor,
}

#[derive(Debug, Clone, Copy)]
pub struct ValleyTrainParams {
	pub segment_count: usize,
	/// Scales role depths; set via [`StampStrength`] (default `1.0`).
	pub strength: f32,
}

impl Default for ValleyTrainParams {
	fn default() -> Self {
		Self { segment_count: 3, strength: 1.0 }
	}
}

impl StampStrength for ValleyTrainParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.strength = strength.max(0.0);
		self
	}
}

#[derive(Debug, Clone)]
pub struct ValleyTrainSegment {
	pub role: ValleyTrainSegmentRole,
	pub basin: ValleyBasin,
	/// `true` when this segment should bind an active channel overlay later.
	pub active_channel: bool,
}

#[derive(Debug, Clone)]
pub struct ValleyTrain {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: ValleyTrainParams,
	pub spine: Vec<Vec2>,
	pub segments: Vec<ValleyTrainSegment>,
	pub stamp: StampSet,
}

impl ValleyTrain {
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: ValleyTrainParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let (start, end) = FractalAnchors::default().sample(bounds, seed, 700);
		let spine = HysteresisSpine::default().build(bounds, seed.wrapping_add(70), start, end);
		let n = params.segment_count.clamp(2, 5);
		let mut segments = Vec::with_capacity(n);
		let mut stamp = StampSet::empty();
		stamp.semantics = StampSemantics::default()
			.with_tag("valley_train")
			.with_drainage_id(seed.wrapping_mul(0xC2B2_AE35));

		for i in 0..n {
			let role = match i {
				0 => ValleyTrainSegmentRole::UpperGorge,
				x if x + 1 == n => ValleyTrainSegmentRole::LowerFloor,
				_ => ValleyTrainSegmentRole::MiddleGlide,
			};
			let (cross, depth, width_frac, active_channel) = match role {
				ValleyTrainSegmentRole::UpperGorge => (ValleyCrossSection::V, 16.0, 0.16, true),
				ValleyTrainSegmentRole::MiddleGlide => (ValleyCrossSection::U, 12.0, 0.22, true),
				ValleyTrainSegmentRole::LowerFloor => (ValleyCrossSection::U, 8.0, 0.28, false),
			};
			let depth = scale_additive(depth, params.strength);
			let t0 = i as f32 / n as f32;
			let t1 = (i + 1) as f32 / n as f32;
			let seg_bounds = sub_bounds_along_spine(bounds, &spine, t0, t1);
			let basin = ValleyBasin::from_bounds(
				seg_bounds,
				seed.wrapping_add(80 + i as u32),
				ValleyBasinParams {
					cross_section: cross,
					floor: ValleyFloorKind::SpillwayReady,
					width_frac,
					depth,
					floor_scale: 1.0,
				},
				height_at,
			);
			let mut seg_stamp = basin.stamp.clone();
			seg_stamp.semantics = seg_stamp.semantics.with_tag(match role {
				ValleyTrainSegmentRole::UpperGorge => "upper_gorge",
				ValleyTrainSegmentRole::MiddleGlide => "middle_glide",
				ValleyTrainSegmentRole::LowerFloor => "lower_floor",
			});
			seg_stamp.semantics = if active_channel {
				seg_stamp.semantics.with_tag("active_channel")
			} else {
				seg_stamp.semantics.with_tag("floodplain_only")
			};
			stamp.extend_with(seg_stamp);
			segments.push(ValleyTrainSegment { role, basin, active_channel });
		}
		stamp.spine = spine.clone();

		Self { bounds, seed, params, spine, segments, stamp }
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, ValleyTrainParams::default(), None)
	}
}

fn sub_bounds_along_spine(parent: Bounds2, spine: &[Vec2], t0: f32, t1: f32) -> Bounds2 {
	if spine.len() < 2 {
		return parent;
	}
	let i0 = ((spine.len() - 1) as f32 * t0).floor() as usize;
	let i1 = ((spine.len() - 1) as f32 * t1).ceil() as usize;
	let i1 = i1.max(i0 + 1).min(spine.len() - 1);
	let mut min = spine[i0];
	let mut max = spine[i0];
	for p in &spine[i0..=i1] {
		min = min.min(*p);
		max = max.max(*p);
	}
	let pad = parent.extent().min_element() * 0.08;
	Bounds2::new(parent.project(min - Vec2::splat(pad)), parent.project(max + Vec2::splat(pad)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn train_has_three_segments() -> anyhow::Result<()> {
		let t = ValleyTrain::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 500.0, 500.0), 8);
		assert_eq!(t.segments.len(), 3);
		assert!(t.stamp.semantics.tags.contains(&"valley_train"));
		Ok(())
	}
}
