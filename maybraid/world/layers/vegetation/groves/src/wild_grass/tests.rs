use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = WildGrassCell::distribution();
	assert_eq!(dist.len(), 13);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 1.0);
	assert_eq!(dist.buckets[1].item, Some(WildGrassCell::MeadowGreen));
	assert_eq!(dist.buckets[1].weight, 0.4);
	assert_eq!(dist.buckets[2].item, Some(WildGrassCell::GoldenGrass));
	assert_eq!(dist.buckets[2].weight, 0.3);
	assert_eq!(dist.buckets[3].item, Some(WildGrassCell::RedPrairie));
	assert_eq!(dist.buckets[3].weight, 0.2);
	assert_eq!(dist.buckets[4].item, Some(WildGrassCell::BlueTropical));
	assert_eq!(dist.buckets[4].weight, 0.16);
	assert_eq!(dist.buckets[5].item, Some(WildGrassCell::PaleField));
	assert_eq!(dist.buckets[5].weight, 0.2);
	assert_eq!(dist.buckets[6].item, Some(WildGrassCell::BloomingGrass));
	assert_eq!(dist.buckets[6].weight, 0.14);
	assert_eq!(dist.buckets[7].item, Some(WildGrassCell::MeadowGreenPatch));
	assert_eq!(dist.buckets[7].weight, 1.6);
	assert_eq!(dist.buckets[8].item, Some(WildGrassCell::GoldenGrassPatch));
	assert_eq!(dist.buckets[8].weight, 1.2);
	assert_eq!(dist.buckets[9].item, Some(WildGrassCell::RedPrairiePatch));
	assert_eq!(dist.buckets[9].weight, 0.8);
	assert_eq!(dist.buckets[10].item, Some(WildGrassCell::BlueTropicalPatch));
	assert_eq!(dist.buckets[10].weight, 0.64);
	assert_eq!(dist.buckets[11].item, Some(WildGrassCell::PaleFieldPatch));
	assert_eq!(dist.buckets[11].weight, 0.8);
	assert_eq!(dist.buckets[12].item, Some(WildGrassCell::BloomingGrassPatch));
	assert_eq!(dist.buckets[12].weight, 0.56);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = WildGrassCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.65..=0.90).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn patches_outweigh_single_clumps() -> Result<()> {
	let placed_weight = |patch: bool| -> f32 {
		WildGrassCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item
					.is_some_and(|cell| matches!(cell.item(), WildGrassItem::Patch(_)) == patch)
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(
		placed_weight(true) > 2.0 * placed_weight(false),
		"patches should dominate placed weight"
	);
	Ok(())
}

#[test]
fn clump_geometry_follows_authored_bands() -> Result<()> {
	for cell in [
		WildGrassCell::MeadowGreen,
		WildGrassCell::GoldenGrass,
		WildGrassCell::RedPrairie,
		WildGrassCell::BlueTropical,
		WildGrassCell::PaleField,
		WildGrassCell::BloomingGrass,
	] {
		let WildGrassItem::Clump(clump) = cell.item() else {
			anyhow::bail!("expected clump item for {cell:?}");
		};
		assert!(clump.height.start >= 0.50);
		assert!(clump.height.end <= 1.0);
		assert!(clump.width_factor.start > 0.0);
		assert!(clump.width_factor.end <= 0.05, "blades should stay grass-thin");
	}
	Ok(())
}

#[test]
fn patch_wraps_meadow_green_clump() -> Result<()> {
	let WildGrassItem::Patch(patch) = WildGrassCell::MeadowGreenPatch.item() else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, MEADOW_GREEN);
	assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
	assert!(patch.patch_extent_xz.start > 0.0);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// BlueTropical (index 4) rejects elevation 0.45; first-fit falls to PaleField (index 5).
	let prepared =
		WildGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.45, steepness: 0.20 };
	let outcome = prepared.select_from(
		4,
		Vec3::new(5.0, 0.45, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, WildGrassCell::PaleField);
		}
		other => anyhow::bail!("expected PaleField fallback, got {other:?}"),
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
