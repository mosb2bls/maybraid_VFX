//! Dispatch a usage node to the matching expander.

use building_components::{FurnitureNode, FurnitureUsage, FurnitureUsageNode};

use crate::bites_counter::BitesCounterUsage;
use crate::bites_kitchen::BitesKitchenUsage;
use crate::bites_seating::BitesSeatingUsage;

/// Expand one packed usage region into kit slots.
pub fn expand_usage(node: &FurnitureUsageNode) -> Vec<FurnitureNode> {
	match node.kind {
		FurnitureUsage::BitesCounter => BitesCounterUsage::expand(node),
		FurnitureUsage::BitesKitchen => BitesKitchenUsage::expand(node),
		FurnitureUsage::BitesSeating => BitesSeatingUsage::expand(node),
	}
}

/// Flatten many usage regions into one slot list.
pub fn expand_usages(nodes: impl IntoIterator<Item = FurnitureUsageNode>) -> Vec<FurnitureNode> {
	nodes.into_iter().flat_map(|node| expand_usage(&node)).collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use building_components::{FurnitureGeometry, Placement};

	#[test]
	fn dispatch_covers_both_kinds() {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(8.0, 3.5, 6.0));
		let mut counter = FurnitureUsageNode::bites_counter(Placement::IDENTITY);
		counter.stamp_region(
			&Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 0.0), Vec3::new(5.0, 3.5, 0.8)),
			&host,
		);
		let mut kitchen = FurnitureUsageNode::bites_kitchen(Placement::IDENTITY);
		kitchen.stamp_region(
			&Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 1.5), Vec3::new(8.0, 3.5, 6.0)),
			&host,
		);
		let slots = expand_usages([counter, kitchen]);
		assert!(slots.iter().any(|n| n.geometry == FurnitureGeometry::Counter));
		assert!(slots.iter().any(|n| n.geometry == FurnitureGeometry::FoodDisplay
			|| n.geometry == FurnitureGeometry::Range
			|| n.geometry == FurnitureGeometry::Shelf));
	}
}
