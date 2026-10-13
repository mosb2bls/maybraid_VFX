use super::*;
use anyhow::Result;

fn small_grove() -> ForlornSavanna {
	ForlornSavannaParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)))
		.build()
}

fn plant_height(plant: &ForlornSavannaPlant) -> f32 {
	match &plant.kind {
		ForlornSavannaKind::Rory(t) => t.geometry.height(),
		ForlornSavannaKind::Bush(t) => t.shape.height,
		ForlornSavannaKind::Storybook(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &ForlornSavannaPlant) -> i32 {
	match &plant.kind {
		ForlornSavannaKind::Rory(t) => t.geometry.canopy_noise.seed,
		ForlornSavannaKind::Bush(t) => t.shape.chain_noise.seed,
		ForlornSavannaKind::Storybook(t) => t.geometry.canopy_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"forlorn-savanna plants",
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
	let rory_n = grove
		.plants
		.iter()
		.filter(|plant| matches!(plant.kind, ForlornSavannaKind::Rory(_)))
		.count();
	assert_eq!(grove.proxy_trunks().len(), rory_n, "each Rory trunk has a proxy stick");
	let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).len();
	assert_eq!(low_foliage, grove.canopy_sites().len());
	assert!(low_foliage >= grove.plants.len());
	assert_eq!(
		grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len(),
		low_foliage,
		"sparse savanna keeps one crown per plant through UltraLow"
	);
	match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
		lod::SceneChunk::Primitive { weight, .. } => {
			assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
		}
		lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
		_ => anyhow::bail!("Low forlorn-savanna should emit flattened canopy kits"),
	}
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = ForlornSavannaParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"forlorn-savanna plants",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}
