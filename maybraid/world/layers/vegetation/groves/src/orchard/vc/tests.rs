use super::*;
use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Entity, Transform};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use vegetation_components::{FlattenedComponentsOnly, PlacedVegetation, VegetationComponents};

fn small_grove() -> Orchard {
	OrchardParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(33.0, 1.0, 33.0)))
		.build()
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"orchard trees",
	)?;

	let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &camera,
		current_transform: &camera,
		bounds: &bounds,
	};

	assert_eq!(grove.stick_nodes_for_level(LodSceneLevel::Low).len(), 0);
	let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
	assert_eq!(low_foliage, grove.plants.len());
	assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
	let lod::SceneChunk::Primitive { weight, .. } =
		grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low)
	else {
		anyhow::bail!("Low orchard should emit one flattened canopy collection");
	};
	assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
	Ok(())
}

#[test]
fn high_tile_fulfill_keeps_full_canopy_when_plants_are_still_medium() -> Result<()> {
	let grove = OrchardParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0)))
		.build();
	let hosts = grove.plants.iter().map(|plant| {
		FlattenedComponentsOnly(PlacedVegetation::new(
			std::sync::Arc::clone(&plant.tree),
			plant.placement,
			plant.stick_material.clone(),
			plant.ball_material.clone(),
			plant.frond_material.clone(),
		))
	});
	crate::grove::woody_checks::assert_high_fulfill_uses_full_canopy(&grove, hosts, "orchard trees")
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	use std::sync::Arc;

	let mut params = OrchardParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"orchard trees",
		|p| p.tree.geometry.height(),
		|p| p.tree.geometry.canopy_noise.seed,
	)?;
	crate::grove::woody_checks::assert_shared_unit_arcs(&grove.plants, |p| {
		Arc::as_ptr(&p.tree) as *const ()
	})?;
	Ok(())
}
