//! Cheap-ball collection — many placed cheap-ball kits under one foliage LOD parent.
//!
//! Same host idea as [`super::FrondCollection`]: one [`crate::FoliageNode`] and one
//! probe. Whether members bake into a [`scene_ref::MultiSceneMerge`] or stay posed
//! kits is [`crate::CollectionPresent`] on the node.

use bevy_math::Vec3;
use lod::scene::LodSceneLevel;

use crate::lod_band::characteristic_extent_abs;
use crate::placed::Placement;

use super::{COLLECTION_HIGH_METERS, COLLECTION_LOW_METERS, COLLECTION_MEDIUM_METERS};

/// Warm-root cull; aliases [`super::COLLECTION_HIGH_METERS`].
pub const CHEAP_BALL_COLLECTION_HIGH_METERS: f32 = COLLECTION_HIGH_METERS;
/// See [`CHEAP_BALL_COLLECTION_HIGH_METERS`].
pub const CHEAP_BALL_COLLECTION_MEDIUM_METERS: f32 = COLLECTION_MEDIUM_METERS;
/// See [`CHEAP_BALL_COLLECTION_HIGH_METERS`].
pub const CHEAP_BALL_COLLECTION_LOW_METERS: f32 = COLLECTION_LOW_METERS;

/// Many cheap-ball placements, one LOD parent.
#[derive(Debug, Clone, PartialEq)]
pub struct CheapBallCollection {
	pub placements: Vec<Placement>,
	pub center: Vec3,
	pub radius: f32,
}

impl CheapBallCollection {
	pub fn new(placements: impl IntoIterator<Item = Placement>) -> Self {
		Self { placements: placements.into_iter().collect(), center: Vec3::ZERO, radius: 1.0 }
	}

	pub fn with_probe(mut self, center: Vec3, radius: f32) -> Self {
		self.center = center;
		self.radius = radius.max(1e-4);
		self
	}

	pub fn bake_bounds_from_placements(mut self) -> Self {
		if let Some((min, max)) = self.aabb() {
			self.center = (min + max) * 0.5;
			self.radius = ((max - min) * 0.5).max_element().max(1e-4);
		}
		self
	}

	pub fn is_empty(&self) -> bool {
		self.placements.is_empty()
	}

	pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		let mut any = false;
		for placement in &self.placements {
			any = true;
			let r = characteristic_extent_abs(placement);
			let c = placement.translation;
			min = min.min(c - Vec3::splat(r));
			max = max.max(c + Vec3::splat(r));
		}
		any.then_some((min, max))
	}

	pub fn center_and_extent(&self) -> (Vec3, f32) {
		(self.center, self.radius.max(1e-4))
	}

	/// Placements presented at `level` after even thinning.
	pub fn placements_for_level(&self, level: LodSceneLevel) -> Vec<Placement> {
		let n = self.placements.len();
		if n == 0 {
			return Vec::new();
		}
		let target = match level {
			LodSceneLevel::High => n,
			LodSceneLevel::Medium => n.div_ceil(2).max(1),
			LodSceneLevel::Low => n.div_ceil(4).max(1),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				1
			}
		};
		subsample_evenly(&self.placements, target)
	}
}

fn subsample_evenly(items: &[Placement], target: usize) -> Vec<Placement> {
	let n = items.len();
	if n == 0 {
		return Vec::new();
	}
	let target = target.clamp(1, n);
	if target == n {
		return items.to_vec();
	}
	(0..target).map(|k| items[(k * n) / target]).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn ball(at: Vec3) -> Placement {
		Placement::foliage_uniform(at, 0.25)
	}

	#[test]
	fn medium_keeps_half_high_balls() -> Result<()> {
		let collection =
			CheapBallCollection::new((0..8).map(|i| ball(Vec3::new(i as f32, 0.0, 0.0))))
				.bake_bounds_from_placements();
		assert_eq!(collection.placements_for_level(LodSceneLevel::High).len(), 8);
		assert_eq!(collection.placements_for_level(LodSceneLevel::Medium).len(), 4);
		assert_eq!(collection.placements_for_level(LodSceneLevel::Low).len(), 2);
		assert_eq!(collection.placements_for_level(LodSceneLevel::UltraLow).len(), 1);
		Ok(())
	}

	#[test]
	fn bake_bounds_covers_members() -> Result<()> {
		let collection =
			CheapBallCollection::new([ball(Vec3::ZERO), ball(Vec3::new(4.0, 0.0, 0.0))])
				.bake_bounds_from_placements();
		let (center, radius) = collection.center_and_extent();
		assert!(center.x > 1.5 && center.x < 2.5);
		assert!(radius >= 2.0);
		Ok(())
	}
}
