//! Static walk colliders from stamped furniture slots.

use avian3d::prelude::{Collider, RigidBody};
use bevy::ecs::template::template;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use building_components::{FurnitureGeometry, FurnitureNode};
use building_physics::BUILDING_FRICTION;
use lod_avian::PhysicsInteractionLayer;

/// Marks the Fixed compound in a [`crate::FurnitureCell`]'s High scene.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FurnitureWalkCollider;

/// One Fixed compound over the `slots` that block walking; `None` when none do.
pub(crate) fn walk_collider(slots: &[FurnitureNode]) -> Option<Collider> {
	let shapes = walk_shapes(slots);
	(!shapes.is_empty()).then(|| Collider::compound(shapes))
}

/// `collider` as a hidden static child. Slots are world-space under an
/// identity host, so the collider is too.
pub(crate) fn walk_collider_scene(collider: Collider) -> impl Scene + 'static {
	bsn! {
		template_value(Name::new("furniture-walk-collider"))
		FurnitureWalkCollider
		template_value(Transform::IDENTITY)
		template_value(Visibility::Hidden)
		template(|_ctx| Ok(RigidBody::Static))
		template(move |_ctx| Ok(collider.clone()))
		template(|_ctx| Ok(PhysicsInteractionLayer::fixed_layers()))
		template(|_ctx| Ok(BUILDING_FRICTION))
	}
}

fn walk_shapes(slots: &[FurnitureNode]) -> Vec<(Vec3, Quat, Collider)> {
	slots.iter().filter(|slot| blocks_walk(slot.geometry)).map(slot_shape).collect()
}

fn blocks_walk(geometry: FurnitureGeometry) -> bool {
	!matches!(
		geometry,
		FurnitureGeometry::Fruit
			| FurnitureGeometry::Bread
			| FurnitureGeometry::Cookware
			| FurnitureGeometry::Faucet
			| FurnitureGeometry::FoodDisplay
			| FurnitureGeometry::Basin
			| FurnitureGeometry::Range
	)
}

fn slot_shape(node: &FurnitureNode) -> (Vec3, Quat, Collider) {
	let size = node.placement.scale.max(Vec3::splat(0.08));
	(
		node.placement.translation,
		node.placement.rotation(),
		Collider::cuboid(size.x, size.y, size.z),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::Placement;

	#[test]
	fn partitions_and_counters_block_sit_ons_do_not() {
		let partition =
			FurnitureNode::partition(Placement::IDENTITY.with_scale(Vec3::new(2.6, 3.2, 0.22)));
		let fruit = FurnitureNode::fruit(Placement::IDENTITY);
		let shapes = walk_shapes(&[partition, fruit]);
		assert_eq!(shapes.len(), 1);
	}
}
