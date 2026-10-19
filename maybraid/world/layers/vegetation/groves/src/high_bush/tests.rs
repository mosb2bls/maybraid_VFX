use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = HighBushCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(HighBushCell::GreenHighBush),
			Some(HighBushCell::DenseHighBush),
			Some(HighBushCell::DryHighBush),
			Some(HighBushCell::BerryHighBush),
			Some(HighBushCell::CopperCaneHighBush),
		]
	);
	for bucket in &dist.buckets {
		assert!(
			bucket.weight.is_finite() && bucket.weight >= 0.0,
			"weight {} must be finite and nonnegative",
			bucket.weight
		);
	}
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = HighBushCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.16..=0.42).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn bush_geometry_follows_authored_bands() -> Result<()> {
	for cell in [
		HighBushCell::GreenHighBush,
		HighBushCell::DenseHighBush,
		HighBushCell::DryHighBush,
		HighBushCell::BerryHighBush,
		HighBushCell::CopperCaneHighBush,
	] {
		let HighBushItem::Bush(bush) = cell.item();
		assert!(bush.height.start >= 1.00);
		assert!(bush.height.end <= 3.50);
		assert!(*bush.shoot_count.start() >= 6);
		assert!(*bush.shoot_count.end() <= 12);
		assert!(*bush.branch_depth.start() >= 2);
		assert!(*bush.branch_depth.end() <= 5);
		assert!(bush.leaf_radius.start >= 0.05);
		assert!(bush.leaf_radius.end <= 0.14);
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
	crate::grove::populate_checks::assert_populated_grove_is_deterministic_and_non_empty(
		definition(),
		NoiseParams::default(),
		GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0)),
		&FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
	)
}
