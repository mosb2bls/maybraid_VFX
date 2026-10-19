use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = ConiferSaplingCell::distribution();
	assert_eq!(dist.len(), 7);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 5.2);
	assert_eq!(dist.buckets[1].item, Some(ConiferSaplingCell::FriendSapling));
	assert_eq!(dist.buckets[1].weight, 1.0);
	assert_eq!(dist.buckets[2].item, Some(ConiferSaplingCell::NorthernSapling));
	assert_eq!(dist.buckets[2].weight, 1.0);
	assert_eq!(dist.buckets[3].item, Some(ConiferSaplingCell::MossyFriendSapling));
	assert_eq!(dist.buckets[3].weight, 0.35);
	assert_eq!(dist.buckets[4].item, Some(ConiferSaplingCell::ColdNorthernSapling));
	assert_eq!(dist.buckets[4].weight, 0.35);
	assert_eq!(dist.buckets[5].item, Some(ConiferSaplingCell::BrightFriendSapling));
	assert_eq!(dist.buckets[5].weight, 0.30);
	assert_eq!(dist.buckets[6].item, Some(ConiferSaplingCell::WindsweptNorthernSapling));
	assert_eq!(dist.buckets[6].weight, 0.40);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = ConiferSaplingCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.28..=0.48).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let ConiferSaplingItem::FriendsConifer(friend) = ConiferSaplingCell::FriendSapling.item()
	else {
		anyhow::bail!("expected friend sapling item");
	};
	assert_eq!(friend.height, SAPLING_HEIGHT);
	assert_eq!(friend.canopy_density, MODERATE_CANOPY_DENSITY);

	let ConiferSaplingItem::NorthernConifer(northern) = ConiferSaplingCell::NorthernSapling.item()
	else {
		anyhow::bail!("expected northern sapling item");
	};
	assert_eq!(northern.height, SAPLING_HEIGHT);

	let ConiferSaplingItem::NorthernConifer(windswept) =
		ConiferSaplingCell::WindsweptNorthernSapling.item()
	else {
		anyhow::bail!("expected windswept northern item");
	};
	assert_eq!(windswept.canopy_density, SPARSE_TO_MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
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
	let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.30 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
