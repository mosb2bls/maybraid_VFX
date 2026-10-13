use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use lod::scene::LodScene;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = TropicalThicketCell::distribution();
	assert_eq!(dist.len(), 7);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 7.0);
	assert_eq!(dist.buckets[1].item, Some(TropicalThicketCell::LargePalmBush));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(TropicalThicketCell::BroadWetPalmBush));
	assert_eq!(dist.buckets[2].weight, 1.25);
	assert_eq!(dist.buckets[3].item, Some(TropicalThicketCell::MiniHonuBanyan));
	assert_eq!(dist.buckets[3].weight, 0.45);
	assert_eq!(dist.buckets[4].item, Some(TropicalThicketCell::ModerateHighBush));
	assert_eq!(dist.buckets[4].weight, 1.0);
	assert_eq!(dist.buckets[5].item, Some(TropicalThicketCell::FloweringHighBush));
	assert_eq!(dist.buckets[5].weight, 0.30);
	assert_eq!(dist.buckets[6].item, Some(TropicalThicketCell::RedStemPalmBush));
	assert_eq!(dist.buckets[6].weight, 0.25);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = TropicalThicketCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.24..=0.62).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn palm_banyan_and_bush_placed_weights_match_rfc_ratio() -> Result<()> {
	let weight = |kind: &str| -> f32 {
		TropicalThicketCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match (kind, cell.item()) {
					("palm", TropicalThicketItem::Palm(_)) => true,
					("banyan", TropicalThicketItem::Banyan(_)) => true,
					("bush", TropicalThicketItem::Bush(_)) => true,
					_ => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	let palm = weight("palm");
	let banyan = weight("banyan");
	let bush = weight("bush");
	assert!((palm - 3.5).abs() < 1e-4, "expected palm weight 3.5, got {palm}");
	assert!((banyan - 0.45).abs() < 1e-4, "expected banyan weight 0.45, got {banyan}");
	assert!((bush - 1.30).abs() < 1e-4, "expected bush weight 1.30, got {bush}");
	Ok(())
}

#[test]
fn palm_banyan_and_bush_geometry_follows_authored_bands() -> Result<()> {
	let TropicalThicketItem::Palm(large) = TropicalThicketCell::LargePalmBush.item() else {
		anyhow::bail!("expected large palm item");
	};
	assert!(large.height.start >= 3.00);
	assert!(large.height.end <= 6.60);
	assert_eq!(large.frond_count, 7..=12);

	let TropicalThicketItem::Palm(wet) = TropicalThicketCell::BroadWetPalmBush.item() else {
		anyhow::bail!("expected broad wet palm item");
	};
	assert!(wet.height.end <= 7.80);
	assert_eq!(wet.frond_count, 8..=14);

	let TropicalThicketItem::Banyan(banyan) = TropicalThicketCell::MiniHonuBanyan.item() else {
		anyhow::bail!("expected banyan item");
	};
	assert!(banyan.height.start >= 1.80);
	assert!(banyan.height.end <= 3.80);
	assert!(banyan.canopy_spread.start >= 1.20);

	let TropicalThicketItem::Bush(moderate) = TropicalThicketCell::ModerateHighBush.item() else {
		anyhow::bail!("expected moderate bush item");
	};
	assert!(moderate.height.start >= 1.20);
	assert!(moderate.leaf_radius.end <= 0.15);
	assert_eq!(moderate.branch_depth, 2..=5);

	let TropicalThicketItem::Bush(flowering) = TropicalThicketCell::FloweringHighBush.item() else {
		anyhow::bail!("expected flowering bush item");
	};
	assert!(flowering.height.end <= 2.20);
	assert_eq!(flowering.shoot_count, 7..=10);
	assert_eq!(flowering.branch_depth, 2..=5);

	let TropicalThicketItem::Palm(red) = TropicalThicketCell::RedStemPalmBush.item() else {
		anyhow::bail!("expected red stem palm item");
	};
	assert!(red.crown_spread.end <= 6.30);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// LargePalmBush (index 1) rejects steepness 0.30; first-fit falls to BroadWetPalmBush
	// (index 2), which allows steepness up to 0.68.
	let prepared =
		TropicalThicketCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.30 };
	let outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.35, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, TropicalThicketCell::BroadWetPalmBush);
		}
		other => anyhow::bail!("expected BroadWetPalmBush fallback, got {other:?}"),
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
#[test]
fn low_and_ultra_low_emit_canopy_ball_proxies() -> Result<()> {
	use lod::scene::LodSceneLevel;
	use vegetation_components::{FoliageGeometry, VegetationComponents};

	let mut params = TropicalThicketParams::default();
	params.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
	params.terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let grove = params.build();
	assert!(!grove.plants.is_empty());

	// Mixed Low: palm five-chord stars + one cheap ball per banyan / bush.
	let low = grove.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
	let fronds = low.iter().filter(|n| n.geometry.is_frond_collection()).count();
	let balls = low.iter().filter(|n| matches!(n.geometry, FoliageGeometry::CheapBall)).count();
	assert_eq!(fronds % 5, 0, "each palm Low star is five frond collections");
	let palms = fronds / 5;
	assert_eq!(fronds, palms * 5);
	assert_eq!(balls, grove.plants.len() - palms);
	assert_eq!(low.len(), fronds + balls);
	assert!(grove.stick_nodes_for_level(LodSceneLevel::Low).flatten().is_empty());

	let ultra = grove.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten();
	assert!(!ultra.is_empty());
	assert!(ultra.len() <= grove.plants.len());
	assert!(ultra.iter().all(|n| matches!(n.geometry, FoliageGeometry::CheapBall)));
	Ok(())
}

#[cfg(feature = "render")]
#[test]
fn high_nests_one_plant_host_chunk_per_plant() -> Result<()> {
	use bevy::prelude::Transform;
	use lod::lod_ref::LodRef;
	use lod::scene::LodSceneLevel;
	use vegetation_components::VegetationComponents;

	let mut params = TropicalThicketParams::default();
	params.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
	params.terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let grove = params.build();
	assert!(!grove.plants.is_empty());

	let identity = Transform::IDENTITY;
	let bounds = grove.scene_bounds();
	let lod_ref = LodRef {
		entity: bevy::prelude::Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
	let lod::SceneChunk::SubChunks(parts) = high else {
		anyhow::bail!("High tropical thicket should wrap plant chunks");
	};
	assert_eq!(parts.len(), 1, "expected one lazy plant producer");
	assert!(
		grove.foliage_nodes_for_level(LodSceneLevel::High).flatten().is_empty(),
		"High foliage stays on nested plant hosts, not the grove"
	);
	Ok(())
}
