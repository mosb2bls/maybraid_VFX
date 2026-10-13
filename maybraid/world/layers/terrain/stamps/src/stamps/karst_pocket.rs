//! Stamp Karst Pockets (unchained) — [RFC-105 §3.8.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#389-jersey-karst-pockets-small-caves-unchained).
//!
//! Height-oracle dip + semantic cavity tags (full SDF carve is a later volume path).

use crate::config::JitteredCenter;
use crate::modulation::{RegionAffineModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{scale_additive, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KarstNavClass {
	Passable,
	CrawlOnly,
	Hazard,
}

#[derive(Debug, Clone, Copy)]
pub struct KarstPocketParams {
	pub nav: KarstNavClass,
	pub size_frac: f32,
	pub depth: f32,
}

impl Default for KarstPocketParams {
	fn default() -> Self {
		Self { nav: KarstNavClass::CrawlOnly, size_frac: 0.08, depth: 14.0 }
	}
}

impl StampStrength for KarstPocketParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.depth = scale_additive(self.depth, strength);
		self
	}
}

#[derive(Debug, Clone)]
pub struct KarstPocket {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: KarstPocketParams,
	pub mouth: Vec2,
	pub stamp: StampSet,
}

impl KarstPocket {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: KarstPocketParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let mouth = JitteredCenter::default().sample(bounds, seed, 400);
		let radius = short * params.size_frac.clamp(0.03, 0.18);
		let noise = RegionNoise::from_seed(seed.wrapping_add(6), 0.06, radius * 0.12);
		let region = Region2D::Circle(CircleRegion { center: mouth, radius });
		let dip = RegionAffineModulation::new(
			region,
			0.35 + 0.2 * hash.unit(2),
			-params.depth,
			radius * 0.25,
			radius * 0.9,
		)
		.with_noise(noise);

		let mut semantics = StampSemantics::default()
			.with_tag("karst")
			.with_tag("cavity")
			.with_tag("entrance");
		semantics = match params.nav {
			KarstNavClass::Passable => semantics.with_tag("passable"),
			KarstNavClass::CrawlOnly => semantics.with_tag("crawl_only"),
			KarstNavClass::Hazard => semantics.with_tag("hazard"),
		};

		Self {
			bounds,
			seed,
			params,
			mouth,
			stamp: StampSet {
				modulations: vec![StampModulation::Affine(dip)],
				spine: vec![mouth],
				semantics,
			},
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, KarstPocketParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn karst_depresses_mouth() -> anyhow::Result<()> {
		let k = KarstPocket::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 200.0, 200.0), 2);
		let h = k.stamp.apply_elevation(60.0, k.mouth.x, k.mouth.y);
		assert!(h < 60.0);
		Ok(())
	}
}
