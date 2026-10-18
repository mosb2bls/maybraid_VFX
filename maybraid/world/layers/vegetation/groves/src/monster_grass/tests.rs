use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = MonsterGrassCell::distribution();
	assert_eq!(dist.len(), 9);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 1.5);
	assert_eq!(dist.buckets[1].item, Some(MonsterGrassCell::GiantWetBlade));
	assert_eq!(dist.buckets[1].weight, 0.40);
	assert_eq!(dist.buckets[5].item, Some(MonsterGrassCell::GiantWetBladePatch));
	assert_eq!(dist.buckets[5].weight, 1.60);
	assert_eq!(dist.buckets[8].item, Some(MonsterGrassCell::RedRibbedBladePatch));
	assert_eq!(dist.buckets[8].weight, 0.28);
	Ok(())
}

#[test]
fn placed_share_matches_dense_understory_target() -> Result<()> {
	let dist = MonsterGrassCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!(
		(0.70..=0.80).contains(&share),
		"placed share {share} outside dense understory band (~75 %)"
	);
	Ok(())
}

#[test]
fn patches_outweigh_single_clumps() -> Result<()> {
	let placed_weight = |multi: bool| -> f32 {
		MonsterGrassCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| {
					let patch = cell.patch();
					(*patch.clump_count.end() > 1) == multi
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(
		placed_weight(true) > 2.0 * placed_weight(false),
		"multi-clump patches should dominate placed weight"
	);
	Ok(())
}

#[test]
fn palette_mix_keeps_authored_color_slots() -> Result<()> {
	for cell in [
		MonsterGrassCell::GiantWetBlade,
		MonsterGrassCell::BroadJungleBlade,
		MonsterGrassCell::PaleGiantReed,
		MonsterGrassCell::RedRibbedBlade,
		MonsterGrassCell::GiantWetBladePatch,
	] {
		let palette = cell.palette_mix();
		assert!(!palette.slots.is_empty(), "expected palette slots for {cell:?}");
		for slot in palette.slots {
			assert!(!slot.start.0.is_empty(), "empty start token for {cell:?}");
			assert!(!slot.end.0.is_empty(), "empty end token for {cell:?}");
		}
	}
	Ok(())
}

#[test]
fn bend_segments_match_tuft_patch_budget() -> Result<()> {
	for cell in [
		MonsterGrassCell::GiantWetBlade,
		MonsterGrassCell::BroadJungleBlade,
		MonsterGrassCell::PaleGiantReed,
		MonsterGrassCell::RedRibbedBlade,
		MonsterGrassCell::GiantWetBladePatch,
	] {
		let segs = &cell.patch().clump.bend_segments;
		assert!(*segs.start() >= 1);
		assert!(*segs.end() <= 3, "{cell:?} bend_segments {segs:?} exceeds 1..=3");
	}
	Ok(())
}

#[test]
fn single_cells_are_one_clump_patches() -> Result<()> {
	for cell in [
		MonsterGrassCell::GiantWetBlade,
		MonsterGrassCell::BroadJungleBlade,
		MonsterGrassCell::PaleGiantReed,
		MonsterGrassCell::RedRibbedBlade,
	] {
		let patch = cell.patch();
		assert_eq!(*patch.clump_count.start(), 1);
		assert_eq!(*patch.clump_count.end(), 1);
		assert!(patch.clump.height.start >= 2.0);
		assert!(patch.clump.height.end <= 6.0);
	}
	Ok(())
}

#[test]
fn patch_wraps_giant_wet_blade_clump() -> Result<()> {
	let patch = MonsterGrassCell::GiantWetBladePatch.patch();
	assert_eq!(patch.clump, GIANT_WET_BLADE_CLUMP);
	assert!(*patch.clump_count.start() >= 3);
	assert!(patch.patch_extent_xz.start >= 1.2);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	let prepared =
		MonsterGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.55 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.35, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, MonsterGrassCell::RedRibbedBlade);
		}
		other => anyhow::bail!("expected RedRibbedBlade fallback, got {other:?}"),
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let placements = grove.populate(&extent, &terrain);
	assert!(!placements.is_empty());

	let cell = definition().cell_extent_xz.x;
	let off_center = placements
		.iter()
		.filter(|p| {
			let local_x = (p.position.x / cell).fract() - 0.5;
			let local_z = (p.position.z / cell).fract() - 0.5;
			local_x.abs() > 0.25 || local_z.abs() > 0.25
		})
		.count();
	assert!(
		off_center * 2 >= placements.len(),
		"expected at least half of {} placements off cell centers, got {off_center}",
		placements.len()
	);
	Ok(())
}

#[test]
fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
	let grove = Grove::assemble(
		definition(),
		ForestGroveBiases::default(),
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}

#[cfg(feature = "render")]
mod render_tests {
	use super::*;
	use crate::grove::placement_noise;
	use crate::monster_grass::MonsterGrassParams;

	#[test]
	fn clump_geometry_builds_within_authored_ranges() -> Result<()> {
		let noise = placement_noise(NoiseParams::default(), Vec3::new(5.0, 0.0, 5.0));
		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
		] {
			let patch = cell.patch();
			let clump = &patch.clump;
			let item = patch.build_tuft_patch(noise);
			assert_eq!(item.clump_count, 1);
			assert!(item.shape.blade_length >= clump.height.start.min(clump.height.end));
			assert!(item.shape.blade_length <= clump.height.start.max(clump.height.end));
			assert!(clump.bend_segments.contains(&item.shape.bend_segments));
			assert!(item.shape.bend_segments <= 3);
		}
		Ok(())
	}

	#[test]
	fn default_does_not_fold() -> Result<()> {
		assert_eq!(MonsterGrassParams::default().merge_collections, 0);
		Ok(())
	}

	#[test]
	fn build_composes_tuft_patches() -> Result<()> {
		use crate::grove::GroveCellVariant;

		let placement =
			GroveCellVariant::new(MonsterGrassCell::GiantWetBlade, Vec3::new(1.0, 0.0, 2.0), 1.0);
		let grove = MonsterGrassParams::with_resolved_placements(
			vec![placement],
			FlatTerrainSample::default(),
			NoiseParams::default(),
		)
		.build();
		assert_eq!(grove.plants.len(), 1);
		assert_eq!(grove.plants[0].patch.clump_count, 1);
		// Unit archetypes keep runs patch-local; world pose lives on the plant placement.
		assert!((grove.plants[0].placement.translation - Vec3::new(1.0, 0.0, 2.0)).length() < 1e-4);
		assert!(grove.plants[0].patch.patch_extent_xz <= 1.0 + 1e-4);
		let base = grove.plants[0].patch.frond_runs()[0].segments[0].placement.translation;
		assert!(
			base.x.abs() < 2.0 && base.z.abs() < 2.0,
			"unit-local blade base should stay near patch origin, got {base:?}"
		);
		Ok(())
	}

	#[test]
	fn patch_variants_quantize_archetypes() -> Result<()> {
		use crate::grove::GroveCellVariant;
		use std::collections::HashSet;

		let placements: Vec<_> = (0..40)
			.map(|i| {
				GroveCellVariant::new(
					MonsterGrassCell::GiantWetBlade,
					Vec3::new(i as f32 * 3.0, 0.0, (i % 5) as f32),
					1.0,
				)
			})
			.collect();
		let mut params = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		);
		params.patch_variants = 4;
		let grove = params.build();
		let seeds: HashSet<i32> = grove.plants.iter().map(|p| p.patch.shape.seed).collect();
		assert!(seeds.len() <= 4, "expected ≤4 unique unit seeds, got {}", seeds.len());
		let same_seed: Vec<_> = grove
			.plants
			.iter()
			.filter(|p| p.patch.shape.seed == grove.plants[0].patch.shape.seed)
			.collect();
		assert!(same_seed.len() >= 2);
		assert!(
			std::sync::Arc::ptr_eq(&same_seed[0].patch, &same_seed[1].patch),
			"same variant should share one cached TuftPatch Arc"
		);
		Ok(())
	}

	#[test]
	fn build_without_fold_keeps_one_plant_per_placement() -> Result<()> {
		use crate::grove::GroveCellVariant;

		let placements: Vec<_> = (0..12)
			.map(|i| {
				GroveCellVariant::new(
					MonsterGrassCell::GiantWetBlade,
					Vec3::new(i as f32, 0.0, 0.0),
					1.0,
				)
			})
			.collect();
		let grove = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		)
		.build();
		assert_eq!(grove.plants.len(), 12);
		Ok(())
	}

	#[test]
	fn build_merges_down_to_collection_cap() -> Result<()> {
		use crate::grove::{GroveCellVariant, GroveExtent};

		// 4×4 placements on a matching extent; merge 4 → 2×2 square bins.
		let placements: Vec<_> = (0..16)
			.map(|i| {
				GroveCellVariant::new(
					MonsterGrassCell::GiantWetBlade,
					Vec3::new((i % 4) as f32 * 5.0 + 2.5, 0.0, (i / 4) as f32 * 5.0 + 2.5),
					1.0,
				)
			})
			.collect();
		let mut params = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		);
		params.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
		params.merge_collections = 4;
		let grove = params.build();
		assert_eq!(grove.plants.len(), 4);
		Ok(())
	}

	#[test]
	fn fold_bins_are_square_not_strips() -> Result<()> {
		use crate::grove::{GroveCellVariant, GroveExtent, DEFAULT_GROVE_EXTENT_XZ};

		let placements = vec![
			GroveCellVariant::new(MonsterGrassCell::GiantWetBlade, Vec3::new(5.0, 0.0, 5.0), 1.0),
			GroveCellVariant::new(MonsterGrassCell::GiantWetBlade, Vec3::new(5.0, 0.0, 95.0), 1.0),
			GroveCellVariant::new(MonsterGrassCell::GiantWetBlade, Vec3::new(95.0, 0.0, 5.0), 1.0),
		];
		let mut params = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		);
		params.extent = GroveExtent::new(
			Vec3::ZERO,
			Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
		);
		params.merge_collections = 4;
		let grove = params.build();
		assert_eq!(grove.plants.len(), 3, "opposite corners must not share an X-strip bin");
		Ok(())
	}

	#[test]
	fn high_collections_present_as_merged_kits() -> Result<()> {
		use crate::grove::GroveCellVariant;
		use bevy::math::bounding::Aabb3d;
		use bevy::prelude::{Entity, Transform};
		use lod::lod_ref::LodRef;
		use lod::scene::{LodScene, LodSceneLevel};
		use lod::SceneChunk;
		use vegetation_components::{
			CollectionPresent, VegetationComponents, FLATTENED_KIT_CHUNK_WEIGHT,
		};

		let grove = MonsterGrassParams::with_resolved_placements(
			vec![GroveCellVariant::new(
				MonsterGrassCell::GiantWetBlade,
				Vec3::new(1.0, 0.0, 2.0),
				1.0,
			)],
			FlatTerrainSample::default(),
			NoiseParams::default(),
		)
		.build();
		let nodes = grove.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!nodes.is_empty());
		assert!(nodes.iter().all(|n| n.collection_present == CollectionPresent::Merge));

		let camera = Transform::from_translation(Vec3::new(1.0, 2.0, 8.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } =
			grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High)
		else {
			anyhow::bail!("High should emit lazy flattened kits, not nested collection hosts");
		};
		assert_eq!(remaining_primitives, nodes.len());
		assert_eq!(remaining_weight, nodes.len() as u32 * FLATTENED_KIT_CHUNK_WEIGHT);
		Ok(())
	}

	#[test]
	fn palette_resolves_to_authored_color() -> Result<()> {
		use crate::grove::WithPalette;
		use bevy::prelude::StandardMaterial;

		for cell in [
			MonsterGrassCell::GiantWetBlade,
			MonsterGrassCell::BroadJungleBlade,
			MonsterGrassCell::PaleGiantReed,
			MonsterGrassCell::RedRibbedBlade,
			MonsterGrassCell::GiantWetBladePatch,
		] {
			let palette = cell.palette_mix();
			let mut allowed = Vec::new();
			for slot in palette.slots {
				allowed.extend(slot.start.resolve());
				allowed.extend(slot.end.resolve());
			}
			assert!(!allowed.is_empty(), "unresolved palette tokens for {cell:?}");
			let material = StandardMaterial::with_palette(StandardMaterial::default(), palette, 7);
			assert!(allowed.contains(&material.base_color));
		}
		Ok(())
	}

	#[test]
	fn structural_lod_thins_to_proxy_grids() -> Result<()> {
		use crate::grove::{GroveCellVariant, DEFAULT_GROVE_EXTENT_XZ};
		use crate::monster_grass::{
			MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR, MONSTER_GRASS_STRUCTURAL_LOW_FACTOR,
			MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR,
		};
		use lod::scene::LodSceneLevel;
		use vegetation_components::VegetationComponents;

		let placements: Vec<_> = (0..8)
			.map(|i| {
				GroveCellVariant::new(
					MonsterGrassCell::GiantWetBlade,
					Vec3::new((i % 4) as f32 * 5.0, 0.0, (i / 4) as f32 * 5.0),
					1.0,
				)
			})
			.collect();
		let grove = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		)
		.with_extent(GroveExtent::new(
			Vec3::ZERO,
			Vec3::new(DEFAULT_GROVE_EXTENT_XZ, 1.0, DEFAULT_GROVE_EXTENT_XZ),
		))
		.build();

		let high_n = grove.foliage_nodes_for_level(LodSceneLevel::High).len();
		let medium_n = grove.foliage_nodes_for_level(LodSceneLevel::Medium).len();
		assert!(high_n >= 1);
		// Medium keeps every 4th plant (~¼ of High tufts).
		assert_eq!(medium_n, high_n.div_ceil(4));
		assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::Low).len(), 1);
		assert_eq!(grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).len(), 4);

		let low_runs = grove
			.foliage_nodes_for_level(LodSceneLevel::Low)
			.flatten()
			.first()
			.and_then(|n| n.geometry.as_frond_collection().map(|c| c.runs.len()))
			.unwrap_or(0);
		// Low bins (~√8 cells ≈ 7.1 m) merge the 5 m lattice → 3 occupied bins.
		assert_eq!(low_runs, 3);

		let probe = grove.structural_lod().expect("probe");
		assert!((probe.high_factor - MONSTER_GRASS_STRUCTURAL_HIGH_FACTOR).abs() < 1e-5);
		assert!((probe.medium_factor - MONSTER_GRASS_STRUCTURAL_MEDIUM_FACTOR).abs() < 1e-5);
		assert!((probe.low_factor - MONSTER_GRASS_STRUCTURAL_LOW_FACTOR).abs() < 1e-5);
		assert!(probe.preserve_ultra_low);
		Ok(())
	}

	#[test]
	fn medium_keeps_quarter_of_high_tufts() -> Result<()> {
		use crate::grove::GroveCellVariant;
		use lod::scene::LodSceneLevel;
		use vegetation_components::VegetationComponents;

		let placements: Vec<_> = (0..16)
			.map(|i| {
				let ix = i % 4;
				let iz = i / 4;
				GroveCellVariant::new(
					MonsterGrassCell::GiantWetBlade,
					Vec3::new(ix as f32 * 2.5 + 1.25, 0.0, iz as f32 * 2.5 + 1.25),
					1.0,
				)
			})
			.collect();
		let grove = MonsterGrassParams::with_resolved_placements(
			placements,
			FlatTerrainSample::default(),
			NoiseParams::default(),
		)
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0)))
		.build();

		let high_n = grove.foliage_nodes_for_level(LodSceneLevel::High).len();
		let medium_n = grove.foliage_nodes_for_level(LodSceneLevel::Medium).len();
		assert_eq!(high_n, 16);
		assert_eq!(medium_n, 4);
		Ok(())
	}
}
