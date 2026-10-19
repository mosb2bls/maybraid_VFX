use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = RiverineGreenCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(RiverineGreenCell::WetGreenBush),
			Some(RiverineGreenCell::BrightBankBush),
			Some(RiverineGreenCell::DeepShadeBush),
			Some(RiverineGreenCell::PaleRiparianBush),
			Some(RiverineGreenCell::RedTwigRiverBush),
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
fn placed_share_matches_moderate_riparian_target() -> Result<()> {
	let dist = RiverineGreenCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!(
		(0.25..=0.35).contains(&share),
		"placed share {share} outside moderate riparian band (~29 %)"
	);
	Ok(())
}

#[test]
fn bush_geometry_follows_authored_bands() -> Result<()> {
	for cell in [
		RiverineGreenCell::WetGreenBush,
		RiverineGreenCell::BrightBankBush,
		RiverineGreenCell::DeepShadeBush,
		RiverineGreenCell::PaleRiparianBush,
		RiverineGreenCell::RedTwigRiverBush,
	] {
		let RiverineGreenItem::Bush(bush) = cell.item();
		assert!(bush.height.start >= 0.80);
		assert!(bush.height.end <= 2.40);
		assert!(*bush.shoot_count.start() >= 6);
		assert!(*bush.shoot_count.end() <= 12);
		assert!(bush.leaf_radius.start >= 0.05);
		assert!(bush.leaf_radius.end <= 0.14);
		assert!(bush.radial_strength.start >= 0.30);
		assert!(bush.radial_strength.end <= 0.58);
		assert!(bush.vertical_bias.start >= 0.18);
		assert!(bush.vertical_bias.end <= 0.90);
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
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
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
