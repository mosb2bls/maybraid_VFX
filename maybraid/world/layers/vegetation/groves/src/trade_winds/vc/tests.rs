use super::*;
use anyhow::Result;

fn small_grove() -> TradeWinds {
	TradeWindsParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)))
		.build()
}

fn plant_height(plant: &TradeWindsPlant) -> f32 {
	match &plant.kind {
		TradeWindsKind::Storybook(t) => t.geometry.height(),
		TradeWindsKind::Honu(t) => t.geometry.scale.tree_height,
		TradeWindsKind::Sope(t) => t.geometry.scale.stalk_height,
		TradeWindsKind::Waialea(t) => t.geometry.height(),
	}
}

fn plant_seed(plant: &TradeWindsPlant) -> i32 {
	match &plant.kind {
		TradeWindsKind::Storybook(t) => t.geometry.canopy_noise.seed,
		TradeWindsKind::Honu(t) => t.geometry.canopy_noise.seed,
		TradeWindsKind::Sope(t) => t.geometry.canopy_noise.seed,
		TradeWindsKind::Waialea(t) => t.geometry.trunk_noise.seed,
	}
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"trade winds plants",
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
	let low_foliage = grove.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
	let palms = grove
		.plants
		.iter()
		.filter(|p| matches!(p.kind, TradeWindsKind::Waialea(_)))
		.count();
	let fronds = low_foliage.iter().filter(|n| n.geometry.is_frond_collection()).count();
	assert_eq!(fronds, palms * 5);
	assert!(!grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
	match grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::Low) {
		lod::SceneChunk::Primitive { weight, .. } => {
			assert_eq!(weight, vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT);
		}
		lod::SceneChunk::SubChunks(parts) => assert!(!parts.is_empty()),
		_ => anyhow::bail!("Low trade winds should emit flattened kits"),
	}
	Ok(())
}

#[test]
fn tree_variants_quantize_archetypes() -> Result<()> {
	let mut params = TradeWindsParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"trade winds plants",
		plant_height,
		plant_seed,
	)?;
	Ok(())
}
