use super::*;
use anyhow::Result;

fn small_grove() -> TemperateMassives {
	TemperateMassivesParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)))
		.build()
}

fn plant_height(plant: &TemperateMassivesPlant) -> f32 {
	match &plant.kind {
		TemperateMassivesKind::Oak(t) => t.geometry.height(),
		TemperateMassivesKind::Storybook(t) => t.geometry.height(),
		TemperateMassivesKind::Rory(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &TemperateMassivesPlant) -> i32 {
	match &plant.kind {
		TemperateMassivesKind::Oak(t) => t.geometry.canopy_noise.seed,
		TemperateMassivesKind::Storybook(t) => t.geometry.canopy_noise.seed,
		TemperateMassivesKind::Rory(t) => t.geometry.canopy_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"temperate massives",
	)?;

	let camera = Transform::from_translation(Vec3::new(40.0, 2.0, 40.0));
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &camera,
		current_transform: &camera,
		bounds: &bounds,
	};

	assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).len() <= 1);
	let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
	assert_eq!(low_foliage, grove.canopy_sites().len());
	assert!(low_foliage >= grove.plants.len());
	assert!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len() <= low_foliage);
	match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
		lod::SceneChunk::Primitive { weight, .. } => {
			assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
		}
		lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
		_ => anyhow::bail!("Low temperate massives should emit flattened canopy kits"),
	}
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = TemperateMassivesParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(250.0, 1.0, 250.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"temperate massives",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}
