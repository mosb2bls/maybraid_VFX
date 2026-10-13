use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = LowBushCell::distribution();
	assert_eq!(dist.len(), 6);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 10.0);
	assert_eq!(dist.buckets[1].item, Some(LowBushCell::GreenLowBush));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(LowBushCell::DryLowBush));
	assert_eq!(dist.buckets[2].weight, 1.0);
	assert_eq!(dist.buckets[3].item, Some(LowBushCell::LeafyLowBush));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(LowBushCell::FloweringLowBush));
	assert_eq!(dist.buckets[4].weight, 0.35);
	assert_eq!(dist.buckets[5].item, Some(LowBushCell::RedStemLowBush));
	assert_eq!(dist.buckets[5].weight, 0.25);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = LowBushCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.45).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn bush_geometry_follows_authored_bands() -> Result<()> {
	for cell in [
		LowBushCell::GreenLowBush,
		LowBushCell::DryLowBush,
		LowBushCell::LeafyLowBush,
		LowBushCell::FloweringLowBush,
		LowBushCell::RedStemLowBush,
	] {
		let LowBushItem::Bush(bush) = cell.item();
		assert!(bush.height.start >= 0.50);
		assert!(bush.height.end <= 1.50);
		assert!(*bush.shoot_count.start() >= 4);
		assert!(*bush.shoot_count.end() <= 10);
		assert!(*bush.branch_depth.start() >= 1);
		assert!(*bush.branch_depth.end() <= 3);
		assert!(bush.leaf_radius.start >= 0.03);
		assert!(bush.leaf_radius.end <= 0.10);
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// LeafyLowBush (index 3) rejects steepness 0.40; first-fit falls to FloweringLowBush
	// (index 4), which allows steepness up to 0.65.
	let prepared =
		LowBushCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.40 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, LowBushCell::FloweringLowBush);
		}
		other => anyhow::bail!("expected FloweringLowBush fallback, got {other:?}"),
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.15 };
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
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
