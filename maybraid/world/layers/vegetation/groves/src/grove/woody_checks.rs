//! Shared woody grove render checks. Per-grove tests supply the built grove and
//! plant accessors; High/Medium nesting and archetype quantization live here.

use std::collections::HashSet;

use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Entity, Transform, Vec3};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel};
use vegetation_components::{FoliageGeometry, VegetationComponents};

/// Camera used by the copied High/Medium nest checks.
pub fn preview_lod_ref<'a>(camera: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
	LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: camera,
		current_transform: camera,
		bounds,
	}
}

fn preview_camera_and_bounds() -> (Transform, Aabb3d) {
	(
		Transform::from_translation(Vec3::new(40.0, 2.0, 40.0)),
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE),
	)
}

/// Cheap-ball / frond members a posed plant emits at `level` (after collection thinning).
pub fn foliage_kit_member_count(
	vegetation: &impl VegetationComponents,
	level: LodSceneLevel,
) -> usize {
	vegetation
		.foliage_nodes_for_level(level)
		.flatten()
		.into_iter()
		.map(|node| match &node.geometry {
			FoliageGeometry::CheapBallCollection(collection) => {
				collection.placements_for_level(level).len()
			}
			FoliageGeometry::FrondCollection(collection) => {
				collection.members_for_level(level).len()
			}
			FoliageGeometry::CheapBall
			| FoliageGeometry::LayeredBall
			| FoliageGeometry::StraightFrond
			| FoliageGeometry::StraightFrondSegment => 1,
		})
		.sum()
}

/// Grove High fulfill must pose High kits even when the viewer is still outside
/// each plant's own High band. Per-tree sampling baked Medium/Low canopy at the
/// tile High edge; those kits never upgraded because plants are not hosts.
pub fn assert_high_fulfill_uses_full_canopy<G, H>(
	grove: &G,
	hosts: impl IntoIterator<Item = H>,
	label: &str,
) -> Result<()>
where
	G: VegetationComponents + LodScene,
	H: VegetationComponents + LodScene,
{
	// Short tile High is 6×50 m = 300 m; mid is 400 m. Stand inside that
	// edge so the tile is High while typical storybook plants (10× radius)
	// are still Medium/Low.
	let tile = grove.scene_bounds();
	let center = (Vec3::from(tile.min) + Vec3::from(tile.max)) * 0.5;
	let camera = Transform::from_translation(center + Vec3::new(275.0, 2.0, 0.0));
	let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
	let lod_ref = preview_lod_ref(&camera, &bounds);
	anyhow::ensure!(
		grove.scene_lod_level(&lod_ref) == LodSceneLevel::High,
		"{label} tile should still be High at the fulfill edge"
	);

	let mut below_high = 0usize;
	for host in hosts {
		let plant_level = host.scene_lod_level(&lod_ref);
		if plant_level == LodSceneLevel::High {
			continue;
		}
		let high = foliage_kit_member_count(&host, LodSceneLevel::High);
		let thinned = foliage_kit_member_count(&host, plant_level);
		anyhow::ensure!(
			high > thinned,
			"{label} plant band {plant_level:?} must thin High canopy ({high} vs {thinned})"
		);
		below_high += 1;
	}
	anyhow::ensure!(
		below_high > 0,
		"expected {label} plants still below High at the tile High edge"
	);

	let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
	let lod::SceneChunk::SubChunks(parts) = high else {
		anyhow::bail!("High {label} should wrap plant chunks");
	};
	anyhow::ensure!(parts.len() == 1, "expected one lazy plant producer");
	Ok(())
}

/// High/Medium must pose one lazy kit group per plant, not a nested LOD host.
pub fn assert_high_medium_nests_plants<G>(grove: &G, plant_count: usize, label: &str) -> Result<()>
where
	G: VegetationComponents + LodScene,
{
	if plant_count == 0 {
		anyhow::bail!("expected placed {label} plants");
	}

	anyhow::ensure!(grove.stick_nodes_for_level(LodSceneLevel::High).len() == 0);
	anyhow::ensure!(grove.foliage_nodes_for_level(LodSceneLevel::High).len() == 0);
	anyhow::ensure!(grove.stick_nodes_for_level(LodSceneLevel::Medium).len() == 0);
	anyhow::ensure!(grove.foliage_nodes_for_level(LodSceneLevel::Medium).len() == 0);

	let (camera, bounds) = preview_camera_and_bounds();
	let lod_ref = preview_lod_ref(&camera, &bounds);
	let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
	let lod::SceneChunk::SubChunks(parts) = high else {
		anyhow::bail!("High {label} should wrap plant chunks");
	};
	anyhow::ensure!(parts.len() == 1, "expected one lazy plant producer");
	let lod::SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } = &parts[0] else {
		anyhow::bail!("High {label} plants should be SceneChunk::Lazy");
	};
	anyhow::ensure!(*remaining_primitives == plant_count);
	anyhow::ensure!(
		*remaining_weight == super::vc_compose::flattened_plant_lazy_weight(plant_count),
		"posed plants should charge flattened kit weight"
	);
	Ok(())
}

/// Same cell positions + `tree_variants = n` share archetypal unit meshes.
pub fn assert_quantized_archetypes<P>(
	plants: &[P],
	max_variants: usize,
	label: &str,
	unit_height: impl Fn(&P) -> f32,
	seed: impl Fn(&P) -> i32,
) -> Result<()> {
	if plants.is_empty() {
		anyhow::bail!("expected placed {label} plants");
	}
	for plant in plants {
		let height = unit_height(plant);
		anyhow::ensure!((height - 1.0).abs() < 1e-4, "expected unit height, got {height}");
	}
	let seeds: HashSet<i32> = plants.iter().map(seed).collect();
	anyhow::ensure!(
		seeds.len() <= max_variants,
		"expected ≤{max_variants} unique unit seeds, got {}",
		seeds.len()
	);
	Ok(())
}

/// Repeated variants must share one unit `Arc` (same type + num).
pub fn assert_shared_unit_arcs<P>(plants: &[P], arc_ptr: impl Fn(&P) -> *const ()) -> Result<()> {
	let ptrs: HashSet<_> = plants.iter().map(arc_ptr).collect();
	anyhow::ensure!(plants.len() > ptrs.len(), "expected repeated variants to share one unit Arc");
	Ok(())
}
