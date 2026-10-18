//! Stick collection — many placed stick kits under one stick LOD parent.
//!
//! One [`crate::StickNode`] schedules a [`scene_ref::MultiSceneMerge`] instead of
//! one host per segment. Trunks stay at every band; branches thin like frond runs.

use bevy_math::Vec3;
use lod::scene::LodSceneLevel;

use crate::foliage::collection::{
	COLLECTION_HIGH_METERS, COLLECTION_LOW_METERS, COLLECTION_MEDIUM_METERS,
};
use crate::lod_band::{characteristic_extent_abs, placement_center};
use crate::placed::Placement;
use crate::sticks::geometry::StickGeometry;

/// Warm-root cull bands match foliage collections (absolute meters).
pub const STICK_COLLECTION_HIGH_METERS: f32 = COLLECTION_HIGH_METERS;
/// See [`STICK_COLLECTION_HIGH_METERS`].
pub const STICK_COLLECTION_MEDIUM_METERS: f32 = COLLECTION_MEDIUM_METERS;
/// See [`STICK_COLLECTION_HIGH_METERS`].
pub const STICK_COLLECTION_LOW_METERS: f32 = COLLECTION_LOW_METERS;

/// One placed stick kit inside a [`StickCollection`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StickMember {
	pub geometry: StickGeometry,
	pub placement: Placement,
}

impl StickMember {
	pub fn segment(placement: Placement) -> Self {
		Self { geometry: StickGeometry::Segment, placement }
	}

	pub fn trunk(placement: Placement) -> Self {
		Self { geometry: StickGeometry::Trunk, placement }
	}

	pub fn is_trunk(self) -> bool {
		matches!(self.geometry, StickGeometry::Trunk)
	}
}

/// Many stick kits, one LOD parent.
#[derive(Debug, Clone, PartialEq)]
pub struct StickCollection {
	pub members: Vec<StickMember>,
	pub center: Vec3,
	pub radius: f32,
}

impl StickCollection {
	pub fn new(members: impl IntoIterator<Item = StickMember>) -> Self {
		Self { members: members.into_iter().collect(), center: Vec3::ZERO, radius: 1.0 }
	}

	pub fn with_probe(mut self, center: Vec3, radius: f32) -> Self {
		self.center = center;
		self.radius = radius.max(1e-4);
		self
	}

	pub fn bake_bounds_from_members(mut self) -> Self {
		if let Some((min, max)) = self.aabb() {
			self.center = (min + max) * 0.5;
			self.radius = ((max - min) * 0.5).max_element().max(1e-4);
		}
		self
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}

	pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		let mut any = false;
		for member in &self.members {
			any = true;
			let c = placement_center(&member.placement);
			let e = characteristic_extent_abs(&member.placement);
			min = min.min(c - Vec3::splat(e));
			max = max.max(c + Vec3::splat(e));
		}
		any.then_some((min, max))
	}

	pub fn center_and_extent(&self) -> (Vec3, f32) {
		(self.center, self.radius.max(1e-4))
	}

	/// Members presented at `level`: all trunks plus thinned branches.
	pub fn members_for_level(&self, level: LodSceneLevel) -> Vec<StickMember> {
		let mut trunks = Vec::new();
		let mut branches = Vec::new();
		for member in &self.members {
			if member.is_trunk() {
				trunks.push(*member);
			} else {
				branches.push(*member);
			}
		}
		let keep_branches = match level {
			LodSceneLevel::High => branches.len(),
			LodSceneLevel::Medium => branches.len().div_ceil(2),
			LodSceneLevel::Low => branches.len().div_ceil(4),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				0
			}
		};
		trunks.extend(subsample_evenly(&branches, keep_branches));
		trunks
	}
}

fn subsample_evenly(items: &[StickMember], target: usize) -> Vec<StickMember> {
	let n = items.len();
	if n == 0 || target == 0 {
		return Vec::new();
	}
	let target = target.min(n);
	if target == n {
		return items.to_vec();
	}
	(0..target).map(|k| items[(k * n) / target]).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn trunk(x: f32) -> StickMember {
		StickMember::trunk(
			Placement::new(Vec3::new(x, 0.0, 0.0), 0.0).with_scale(Vec3::new(0.2, 4.0, 0.2)),
		)
	}

	fn branch(x: f32) -> StickMember {
		StickMember::segment(
			Placement::new(Vec3::new(x, 2.0, 0.0), 0.0).with_scale(Vec3::new(0.1, 1.0, 0.1)),
		)
	}

	#[test]
	fn thinning_keeps_trunks_and_drops_branches() -> Result<()> {
		let collection =
			StickCollection::new([trunk(0.0)].into_iter().chain((0..8).map(|i| branch(i as f32))));
		let high = collection.members_for_level(LodSceneLevel::High);
		assert_eq!(high.iter().filter(|m| m.is_trunk()).count(), 1);
		assert_eq!(high.len(), 9);

		let medium = collection.members_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.iter().filter(|m| m.is_trunk()).count(), 1);
		assert_eq!(medium.len(), 5);

		let low = collection.members_for_level(LodSceneLevel::Low);
		assert_eq!(low.iter().filter(|m| m.is_trunk()).count(), 1);
		assert_eq!(low.len(), 3);

		let ultra = collection.members_for_level(LodSceneLevel::UltraLow);
		assert_eq!(ultra.len(), 1);
		assert!(ultra[0].is_trunk());
		Ok(())
	}
}
