use super::*;
use anyhow::Result;

fn small_grove() -> DateGrove {
	DateGroveParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)))
		.build()
}

#[test]
fn high_medium_nest_one_flattened_host_per_tree() -> Result<()> {
	let grove = small_grove();
	crate::grove::woody_checks::assert_high_medium_nests_plants(
		&grove,
		grove.plants.len(),
		"date-grove plants",
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
		anyhow::bail!("Low date-grove should nest plant chunks");
	};
	assert_eq!(parts.len(), 1, "expected one lazy plant producer");
	let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0] else {
		anyhow::bail!("Low date-grove plants should be SceneChunk::Lazy");
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
	use std::sync::Arc;

	let mut params = DateGroveParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)));
	params.tree_variants = 4;
	let grove = params.build();
	crate::grove::woody_checks::assert_quantized_archetypes(
		&grove.plants,
		4,
		"date-grove plants",
		|p| p.tree.geometry.height(),
		|p| p.tree.geometry.trunk_noise.seed,
	)?;
	crate::grove::woody_checks::assert_shared_unit_arcs(&grove.plants, |p| {
		Arc::as_ptr(&p.tree) as *const ()
	})?;
	Ok(())
}
