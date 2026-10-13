//! Shared Avian collision memberships for Maybraid LOD / terrain / motion.
//!
//! Contact pairs require both sides' **memberships** to pass the other's **filters**
//! ([`CollisionLayers::interacts_with`]). [`LayerMask::NONE`] is empty (no layers),
//! not “all” — use [`LayerMask::ALL`] for the inclusive-everything mask.
//!
//! | Layer | Role | Typical filters |
//! | --- | --- | --- |
//! | [`PhysicsInteractionLayer::Host`] | Optional Avian scene-host volumes | none (query-only) |
//! | [`PhysicsInteractionLayer::Projectile`] | Blaster bolts / bullets | none (query-only; sweeps query Fixed) |
//! | [`PhysicsInteractionLayer::Fixed`] | Terrain / buildings | [`Animated`](PhysicsInteractionLayer::Animated) |
//! | [`PhysicsInteractionLayer::Animated`] | Characters / movers | [`Fixed`](PhysicsInteractionLayer::Fixed) + [`Animated`](PhysicsInteractionLayer::Animated) |
//!
//! Generate / present ids use [`lod::hcsg::HcsgStorage`], not Avian query
//! volumes. Scene-host refresh uses Gimme, not Host colliders.

use avian3d::prelude::{Collider, CollisionLayers, LayerMask, PhysicsLayer, SpatialQueryFilter};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::AvianLodSceneBoundsMarshaller;
use lod::LodSceneBoundsMarshaller;

/// Maybraid physics layers (Avian bit 0 is reserved as the engine default layer).
#[derive(PhysicsLayer, Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PhysicsInteractionLayer {
	/// Avian default layer (bit 0). Prefer tagging new colliders explicitly.
	#[default]
	Default,
	/// Optional Avian scene-host volumes ([`AvianLodSceneBoundsMarshaller`]). Query-only.
	Host,
	/// Static world geometry (terrain trimeshes, buildings).
	Fixed,
	/// Dynamic movers (characters, props that should rest on Fixed).
	Animated,
	/// Bolts / bullets. Query-only; they sweep [`Fixed`](Self::Fixed) instead of contacting.
	Projectile,
}

impl PhysicsInteractionLayer {
	/// Scene hosts: member of [`Host`](Self::Host), contacts nobody.
	pub fn host_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Host, LayerMask::NONE)
	}

	/// Projectiles: member of [`Projectile`](Self::Projectile), contacts nobody.
	pub fn projectile_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Projectile, LayerMask::NONE)
	}

	/// Fixed geometry: member of [`Fixed`](Self::Fixed), contacts [`Animated`](Self::Animated) only.
	pub fn fixed_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Fixed, Self::Animated)
	}

	/// Animated movers: member of [`Animated`](Self::Animated), contacts
	/// [`Fixed`](Self::Fixed) and other [`Animated`](Self::Animated) capsules.
	/// Capsule restitution stays zero so this is a shove, not a bounce.
	pub fn animated_layers() -> CollisionLayers {
		CollisionLayers::new(Self::Animated, [Self::Fixed, Self::Animated])
	}

	/// Spatial-query mask that includes only this layer.
	pub fn query_filter(self) -> SpatialQueryFilter {
		SpatialQueryFilter::from_mask(self)
	}
}

/// Query-only cuboid (or compound) + layer membership.
#[derive(Bundle)]
pub struct AvianLodQueryVolume {
	pub collider: Collider,
	pub layers: CollisionLayers,
}

/// Scene-host volume. Same bundle as [`AvianLodQueryVolume`].
pub type AvianLodHostVolume = AvianLodQueryVolume;

pub(crate) fn volume_from_bounds(bounds: Aabb3d, layers: CollisionLayers) -> AvianLodQueryVolume {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let center = (min + max) * 0.5;
	let size = (max - min).max(Vec3::splat(1e-3));
	let cuboid = Collider::cuboid(size.x, size.y, size.z);
	let collider = if center.length_squared() <= 1e-8 {
		cuboid
	} else {
		Collider::compound(vec![(center, Quat::IDENTITY, cuboid)])
	};
	AvianLodQueryVolume { collider, layers }
}

impl LodSceneBoundsMarshaller for AvianLodSceneBoundsMarshaller {
	type Volume = AvianLodQueryVolume;

	fn volume_from_bounds(bounds: Aabb3d) -> Self::Volume {
		volume_from_bounds(bounds, PhysicsInteractionLayer::host_layers())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn query_only_layers() -> [CollisionLayers; 2] {
		[PhysicsInteractionLayer::host_layers(), PhysicsInteractionLayer::projectile_layers()]
	}

	#[test]
	fn lod_query_layers_contact_nobody() {
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		for layer in query_only_layers() {
			assert!(!layer.interacts_with(layer));
			assert!(!layer.interacts_with(fixed));
			assert!(!layer.interacts_with(animated));
			assert!(!fixed.interacts_with(layer));
			assert!(!animated.interacts_with(layer));
		}
	}

	#[test]
	fn lod_query_layers_do_not_contact_each_other() {
		let host = PhysicsInteractionLayer::host_layers();
		let projectile = PhysicsInteractionLayer::projectile_layers();
		assert!(!host.interacts_with(projectile));
		assert!(!projectile.interacts_with(host));
	}

	#[test]
	fn fixed_does_not_contact_fixed() {
		let fixed = PhysicsInteractionLayer::fixed_layers();
		assert!(!fixed.interacts_with(fixed));
	}

	#[test]
	fn fixed_contacts_animated_both_ways() {
		let fixed = PhysicsInteractionLayer::fixed_layers();
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(fixed.interacts_with(animated));
		assert!(animated.interacts_with(fixed));
	}

	#[test]
	fn animated_contacts_animated() {
		let animated = PhysicsInteractionLayer::animated_layers();
		assert!(animated.interacts_with(animated));
	}

	#[test]
	fn host_marshaller_stamps_host_membership() {
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let host = AvianLodSceneBoundsMarshaller::volume_from_bounds(bounds);
		assert_eq!(host.layers, PhysicsInteractionLayer::host_layers());
	}
}
