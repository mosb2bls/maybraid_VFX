use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = SpottyBushesCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 10.0);
	assert_eq!(dist.buckets[1].item, Some(SpottyBushesCell::GreenSpotBush));
	assert_eq!(dist.buckets[1].weight, 1.5);
	assert_eq!(dist.buckets[2].item, Some(SpottyBushesCell::DrySpotBush));
	assert_eq!(dist.buckets[2].weight, 1.0);
	assert_eq!(dist.buckets[3].item, Some(SpottyBushesCell::DenseSpotBush));
	assert_eq!(dist.buckets[3].weight, 0.60);
	assert_eq!(dist.buckets[4].item, Some(SpottyBushesCell::FloweringSpotBush));
	assert_eq!(dist.buckets[4].weight, 0.25);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = SpottyBushesCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.04..=0.26).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn scrub_geometry_follows_authored_bands() -> Result<()> {
	for (cell, min_h, max_h) in [
		(SpottyBushesCell::GreenSpotBush, 1.00_f32, 2.10),
		(SpottyBushesCell::DrySpotBush, 0.80, 1.80),
		(SpottyBushesCell::DenseSpotBush, 1.40, 2.50),
		(SpottyBushesCell::FloweringSpotBush, 0.90, 1.80),
	] {
		let SpottyBushesItem::Bush(bush) = cell.item();
		assert_eq!(bush.height.start, min_h);
		assert_eq!(bush.height.end, max_h);
		assert!(*bush.shoot_count.start() >= 5);
		assert!(*bush.shoot_count.end() <= 12);
		assert!(*bush.branch_depth.start() >= 1);
		assert!(*bush.branch_depth.end() <= 5);
		assert!(bush.leaf_radius.start >= 0.04);
		assert!(bush.leaf_radius.end <= 0.14);
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn dry_spot_bush_accepts_steeper_slope_than_dense() -> Result<()> {
	let prepared =
		SpottyBushesCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.45 };
	let dry_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.35, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match dry_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, SpottyBushesCell::DrySpotBush);
		}
		other => anyhow::bail!("expected DrySpotBush on moderate slope, got {other:?}"),
	}
	let dense_outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.35, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match dense_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, SpottyBushesCell::DenseSpotBush);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_dense_and_flowering() -> Result<()> {
	let prepared =
		SpottyBushesCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.45 };
	for (index, cell) in
		[(3, SpottyBushesCell::DenseSpotBush), (4, SpottyBushesCell::FloweringSpotBush)]
	{
		let outcome = prepared.select_from(
			index,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_ne!(variant, cell, "expected {cell:?} to reject steepness 0.45");
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
		}
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
