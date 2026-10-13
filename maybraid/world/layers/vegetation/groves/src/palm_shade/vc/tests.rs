use super::*;
use anyhow::Result;

fn small_grove() -> PalmShade {
	PalmShadeParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)))
		.build()
}

fn plant_height(plant: &PalmShadePlant) -> f32 {
	match &plant.kind {
		PalmShadeKind::Waialea(t) => t.geometry.height(),
		PalmShadeKind::Date(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &PalmShadePlant) -> i32 {
	match &plant.kind {
		PalmShadeKind::Waialea(t) => t.geometry.trunk_noise.seed,
		PalmShadeKind::Date(t) => t.geometry.trunk_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"palm-shade plants",
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
	assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Low).len(), 0);
	assert!(!grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
	let low = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low);
	let lod::SceneChunk::SubChunks(parts) = low else {
		anyhow::bail!("Low palm-shade should nest plant chunks");
	};
	assert_eq!(parts.len(), 1, "expected one lazy plant producer");
	let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0] else {
		anyhow::bail!("Low palm-shade plants should be SceneChunk::Lazy");
	};
	assert_eq!(*remaining_primitives, grove.plants.len());
	assert_eq!(
		*remaining_weight,
		crate::grove::vc_compose::flattened_plant_lazy_weight(grove.plants.len())
	);
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = PalmShadeParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"palm-shade plants",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}
