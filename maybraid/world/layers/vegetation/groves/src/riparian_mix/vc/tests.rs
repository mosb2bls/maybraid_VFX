use super::*;
use anyhow::Result;

fn small_grove() -> RiparianMix {
	RiparianMixParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)))
		.build()
}

fn plant_height(plant: &RiparianMixPlant) -> f32 {
	match &plant.kind {
		RiparianMixKind::Oak(t) => t.geometry.height(),
		RiparianMixKind::Storybook(t) => t.geometry.height(),
		RiparianMixKind::Friends(t) => t.geometry.height(),
		RiparianMixKind::Temperate(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &RiparianMixPlant) -> i32 {
	match &plant.kind {
		RiparianMixKind::Oak(t) => t.geometry.canopy_noise.seed,
		RiparianMixKind::Storybook(t) => t.geometry.canopy_noise.seed,
		RiparianMixKind::Friends(t) => t.geometry.canopy_noise.seed,
		RiparianMixKind::Temperate(t) => t.geometry.canopy_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"riparian-mix plants",
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
		anyhow::bail!("Low riparian-mix should emit one flattened canopy collection");
	};
	assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = RiparianMixParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"riparian-mix plants",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}
