use super::*;
use anyhow::Result;

fn small_grove() -> BushScrub {
	BushScrubParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)))
		.build()
}

fn plant_unit_size(plant: &BushScrubPlant) -> f32 {
	match &plant.kind {
		BushScrubKind::Tuft(t) => t.patch_extent_xz.max(t.shape.blade_length),
		BushScrubKind::Bush(t) => t.shape.height,
	}
}

fn plant_seed(plant: &BushScrubPlant) -> i32 {
	match &plant.kind {
		BushScrubKind::Tuft(t) => t.shape.seed,
		BushScrubKind::Bush(t) => t.shape.chain_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"bush-scrub plants",
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
	match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
		lod::SceneChunk::Primitive { weight, .. } => {
			assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
		}
		lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
		_ => anyhow::bail!("Low bush-scrub should emit flattened canopy kits"),
	}
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = BushScrubParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)));
	params.patch_variants = 4;
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"bush-scrub plants",
		plant_unit_size,
		plant_seed,
	)?;
	Ok(())
}
