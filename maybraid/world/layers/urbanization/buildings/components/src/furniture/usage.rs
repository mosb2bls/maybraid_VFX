//! Region IR: a packed usage AABB that later expands into an ensemble of
//! [`super::FurnitureNode`]s (not a single stretched kit).

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

use crate::furniture::abutment::FurnitureAbutment;
use crate::placed::Placement;

/// Packed usage region that is **not** itself a furniture kit.
///
/// Richmond packers emit these for labels like `BitesCounter` / `BitesKitchen`.
/// [`furniture-usage-areas`](https://github.com/ramate-io/maybraid) expands them
/// into station counters, ranges, sit-on props, and the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FurnitureUsage {
	/// Passage-face service band (storey-tall plan box, 1 m working slab).
	BitesCounter,
	/// Max-empty remainder behind those bands.
	BitesKitchen,
	/// Sit-down seating pocket (tables + chairs).
	BitesSeating,
}

impl FurnitureUsage {
	/// Discriminant mixed into [`FurnitureUsageNode::finish_seed`].
	pub const fn finish_salt(self) -> u64 {
		match self {
			Self::BitesCounter => 0x6263_6e74,
			Self::BitesKitchen => 0x626b_7463,
			Self::BitesSeating => 0x6273_6561,
		}
	}
}

/// Authoring IR for a furniture **usage region**.
///
/// [`Self::placement`] fills the packed AABB with identity yaw and world-axis
/// scale so expanders can recover [`Self::region_aabb`]. [`Self::abutment`] is
/// the host face the region flushes to (passage wall for a counter band).
#[derive(Debug, Clone, PartialEq)]
pub struct FurnitureUsageNode {
	pub kind: FurnitureUsage,
	pub placement: Placement,
	pub abutment: Option<FurnitureAbutment>,
	pub finish_seed: u64,
}

impl FurnitureUsageNode {
	pub fn new(kind: FurnitureUsage, placement: Placement) -> Self {
		Self { kind, placement, abutment: None, finish_seed: 0 }
	}

	pub fn bites_counter(placement: Placement) -> Self {
		Self::new(FurnitureUsage::BitesCounter, placement)
	}

	pub fn bites_kitchen(placement: Placement) -> Self {
		Self::new(FurnitureUsage::BitesKitchen, placement)
	}

	pub fn bites_seating(placement: Placement) -> Self {
		Self::new(FurnitureUsage::BitesSeating, placement)
	}

	pub fn with_abutment(mut self, abutment: FurnitureAbutment) -> Self {
		self.abutment = Some(abutment);
		self
	}

	pub fn with_finish_seed(mut self, finish_seed: u64) -> Self {
		self.finish_seed = finish_seed;
		self
	}

	/// Fill `region` with identity yaw; record flush against `host`.
	pub fn stamp_region(&mut self, region: &Aabb3d, host: &Aabb3d) {
		self.finish_seed = finish_seed_for_usage(self.kind, region);
		self.placement.translation = Vec3::from((region.min + region.max) * 0.5);
		self.placement.yaw = 0.0;
		self.placement.pitch = 0.0;
		self.placement.roll = 0.0;
		self.placement.scale = Vec3::from(region.max - region.min).max(Vec3::splat(1e-4));
		self.abutment = FurnitureAbutment::from_flush(region, host);
	}

	/// Axis-aligned packed box recovered from [`Self::placement`].
	pub fn region_aabb(&self) -> Aabb3d {
		let half = self.placement.scale * 0.5;
		Aabb3d::from_min_max(self.placement.translation - half, self.placement.translation + half)
	}
}

/// Mix usage kind + region center into a stable finish key.
pub fn finish_seed_for_usage(kind: FurnitureUsage, region: &Aabb3d) -> u64 {
	let center = (region.min + region.max) * 0.5;
	let mut value = kind.finish_salt()
		^ (center.x.to_bits() as u64).rotate_left(7)
		^ (center.y.to_bits() as u64).rotate_left(17)
		^ (center.z.to_bits() as u64).rotate_left(29);
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn stamp_recovers_the_packed_box() {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(8.0, 3.5, 6.0));
		let band = Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 0.0), Vec3::new(5.0, 3.5, 0.8));
		let mut node = FurnitureUsageNode::bites_counter(Placement::IDENTITY);
		node.stamp_region(&band, &host);
		let back = node.region_aabb();
		assert!((back.min.x - band.min.x).abs() < 1e-4);
		assert!((back.max.z - band.max.z).abs() < 1e-4);
		assert_eq!(node.abutment, Some(FurnitureAbutment::NegZ));
		assert_eq!(node.kind, FurnitureUsage::BitesCounter);
	}
}
