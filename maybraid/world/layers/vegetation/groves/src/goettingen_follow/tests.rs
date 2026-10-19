use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = GoettingenFollowCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(GoettingenFollowCell::FollowBraidOak),
			Some(GoettingenFollowCell::RedBranchBraidOak),
			Some(GoettingenFollowCell::MossyTrailBraidOak),
			Some(GoettingenFollowCell::ParkEdgeBraidOak),
			Some(GoettingenFollowCell::TallFollowBraidOak),
			Some(GoettingenFollowCell::OldGrowthFollowBraidOak),
			Some(GoettingenFollowCell::FollowStorybook),
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
	let dist = GoettingenFollowCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.10..=0.28).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let GoettingenFollowItem::BraidOak(oak) = GoettingenFollowCell::FollowBraidOak.item() else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(4.0, 9.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let GoettingenFollowItem::BraidOak(tall) = GoettingenFollowCell::TallFollowBraidOak.item()
	else {
		anyhow::bail!("expected tall braid oak item");
	};
	assert_eq!(tall.height, UnitRange::new(7.0, 11.0));

	let GoettingenFollowItem::BraidOak(old) = GoettingenFollowCell::OldGrowthFollowBraidOak.item()
	else {
		anyhow::bail!("expected old-growth braid oak item");
	};
	assert_eq!(old.height, UnitRange::new(8.0, 12.0));

	let GoettingenFollowItem::Storybook(story) = GoettingenFollowCell::FollowStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(4.0, 9.0));
	assert_eq!(story.canopy_spread, UnitRange::new(1.6, 4.0));
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
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
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
