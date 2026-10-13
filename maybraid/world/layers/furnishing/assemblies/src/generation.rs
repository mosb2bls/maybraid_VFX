//! Pass over Richmond buildings: collect High furniture slots and paint kits.

use building_components::{BuildingComponents, FurnitureNode};
use furniture_usage_areas::expand_usages;
use lod::LodSceneLevel;

use crate::fill::try_assembly;
use crate::Assembly;

/// High-LOD furniture slots on a Richmond building (the generate pass).
///
/// Room packers already stamped abutment, facing, and [`FurnitureNode::finish_seed`].
/// Usage regions (`BitesCounter`, `BitesKitchen`, `BitesSeating`) expand into ensembles here.
/// Residual gallery / walkway leftovers still emit occasional chests as slots.
pub fn collect_furniture_slots(building: &impl BuildingComponents) -> Vec<FurnitureNode> {
	let mut nodes = building.furniture_nodes_for_level(LodSceneLevel::High).flatten();
	nodes.extend(expand_usages(
		building.furniture_usage_nodes_for_level(LodSceneLevel::High).flatten(),
	));
	nodes
}

/// Slots that have a painted assembly (bed / chair / chest / counter).
pub fn generate_assemblies(building: &impl BuildingComponents) -> Vec<(FurnitureNode, Assembly)> {
	generate_assemblies_from_nodes(collect_furniture_slots(building))
}

/// Same filter as [`generate_assemblies`], from an already-collected slot list.
pub fn generate_assemblies_from_nodes(
	nodes: impl IntoIterator<Item = FurnitureNode>,
) -> Vec<(FurnitureNode, Assembly)> {
	nodes
		.into_iter()
		.filter_map(|node| try_assembly(&node).map(|assembly| (node, assembly)))
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::{FurnitureGeometry, FurnitureUsageNode, Layers, Placement};

	struct PackedRoom;

	impl BuildingComponents for PackedRoom {
		fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
			if !matches!(level, LodSceneLevel::High) {
				return Layers::new();
			}
			Layers::from_free(vec![
				FurnitureNode::bed(Placement::IDENTITY).with_finish_seed(3),
				FurnitureNode::dresser(Placement::IDENTITY).with_finish_seed(3),
				FurnitureNode::chair(Placement::IDENTITY).with_finish_seed(7),
			])
		}
	}

	#[test]
	fn pass_collects_high_slots_and_paints_known_kits() -> anyhow::Result<()> {
		if collect_furniture_slots(&PackedRoom).len() != 3 {
			return Err(anyhow::anyhow!("expected three High slots"));
		}
		if !PackedRoom.furniture_nodes_for_level(LodSceneLevel::Medium).flatten().is_empty() {
			return Err(anyhow::anyhow!("Medium should not emit furniture"));
		}
		let painted = generate_assemblies(&PackedRoom);
		if painted.len() != 2 {
			return Err(anyhow::anyhow!("bed and chair should paint, dresser should not"));
		}
		let kinds: Vec<_> = painted.iter().map(|(n, _)| n.geometry).collect();
		if kinds != [FurnitureGeometry::Bed, FurnitureGeometry::Chair] {
			return Err(anyhow::anyhow!("painted kinds drifted: {kinds:?}"));
		}
		Ok(())
	}

	struct CounterBand;

	impl BuildingComponents for CounterBand {
		fn furniture_usage_nodes_for_level(
			&self,
			level: LodSceneLevel,
		) -> Layers<FurnitureUsageNode> {
			if !matches!(level, LodSceneLevel::High) {
				return Layers::new();
			}
			let host = bevy::math::bounding::Aabb3d::from_min_max(
				bevy::math::Vec3::ZERO,
				bevy::math::Vec3::new(8.0, 3.5, 6.0),
			);
			let band = bevy::math::bounding::Aabb3d::from_min_max(
				bevy::math::Vec3::new(1.0, 0.0, 0.0),
				bevy::math::Vec3::new(5.4, 3.5, 0.8),
			);
			let mut node = FurnitureUsageNode::bites_counter(Placement::IDENTITY);
			node.stamp_region(&band, &host);
			Layers::from_free(vec![node])
		}
	}

	#[test]
	fn usage_regions_expand_into_station_slots() -> anyhow::Result<()> {
		let slots = collect_furniture_slots(&CounterBand);
		if slots.len() < 2 {
			return Err(anyhow::anyhow!(
				"BitesCounter should expand to stations, got {}",
				slots.len()
			));
		}
		if !slots.iter().any(|n| n.geometry == FurnitureGeometry::Counter) {
			return Err(anyhow::anyhow!("expanded band should include counters"));
		}
		Ok(())
	}
}
