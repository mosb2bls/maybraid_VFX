//! Stamp Cave Networks (chained caves) — [RFC-105 §3.8.10](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#3810-jersey-cave-networks-chained-caves).
//!
//! Height dips along a passage spine plus semantic tunnel graph tags.
//! Full 3D SDF tunnels are a later volume path.

use crate::config::{FractalAnchors, HysteresisSpine, SoftmaskAlongSpine};
use crate::region::RegionNoise;
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaveSegmentKind {
	Mouth,
	Slot,
	Chamber,
	Sump,
	DaylightExit,
}

#[derive(Debug, Clone, Copy)]
pub struct CaveNetworkParams {
	pub width_frac: f32,
	pub depth: f32,
}

impl Default for CaveNetworkParams {
	fn default() -> Self {
		Self { width_frac: 0.09, depth: 12.0 }
	}
}

impl StampStrength for CaveNetworkParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.depth = scale_additive(self.depth, strength);
		self
	}
}

#[derive(Debug, Clone)]
pub struct CaveSegment {
	pub kind: CaveSegmentKind,
	pub center: Vec2,
}

#[derive(Debug, Clone)]
pub struct CaveNetwork {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: CaveNetworkParams,
	pub tunnel_id: u32,
	pub path: Vec<Vec2>,
	pub segments: Vec<CaveSegment>,
	pub stamp: StampSet,
}

impl CaveNetwork {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: CaveNetworkParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let tunnel_id = seed.wrapping_mul(0x27D4_EB2D);
		let (start, end) = FractalAnchors::default().sample(bounds, seed, 800);
		let path = HysteresisSpine::default().build(bounds, seed.wrapping_add(81), start, end);
		let half_w = short * params.width_frac.clamp(0.04, 0.18);
		let noise = RegionNoise::from_seed(seed.wrapping_add(3), 0.05, half_w * 0.12);
		let modulations = SoftmaskAlongSpine::default().even_for_extent(short).build(
			&path,
			half_w,
			0.4,
			-params.depth,
			0.2,
			0.7,
			&noise,
			Vec2::ZERO,
		);

		let kinds = [
			CaveSegmentKind::Mouth,
			CaveSegmentKind::Slot,
			CaveSegmentKind::Chamber,
			CaveSegmentKind::Sump,
			CaveSegmentKind::DaylightExit,
		];
		let mut segments = Vec::new();
		if !path.is_empty() {
			for (i, kind) in kinds.iter().enumerate() {
				let idx = (i * (path.len() - 1)) / (kinds.len() - 1);
				segments.push(CaveSegment { kind: *kind, center: path[idx] });
			}
		}
		let flooded = hash.unit(4) > 0.55;

		let mut semantics = StampSemantics::default()
			.with_complex_id(tunnel_id)
			.with_tag("cave_network")
			.with_tag("tunnel")
			.with_tag("branch_node")
			.with_tag("air");
		if flooded {
			semantics = semantics.with_tag("flooded_segment");
		}

		Self {
			bounds,
			seed,
			params,
			tunnel_id,
			path: path.clone(),
			segments,
			stamp: StampSet { modulations, spine: path, semantics },
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, CaveNetworkParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cave_has_passage_segments() -> anyhow::Result<()> {
		let c = CaveNetwork::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 360.0, 360.0), 6);
		assert_eq!(c.segments.len(), 5);
		assert_eq!(c.stamp.semantics.complex_id, Some(c.tunnel_id));
		Ok(())
	}
}
