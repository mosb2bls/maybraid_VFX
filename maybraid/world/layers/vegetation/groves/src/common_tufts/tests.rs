use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = CommonTuftsCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(CommonTuftsCell::ShortGreen),
			Some(CommonTuftsCell::DryScrub),
			Some(CommonTuftsCell::TallWild),
			Some(CommonTuftsCell::ShortGreenPatch),
			Some(CommonTuftsCell::DryScrubPatch),
			Some(CommonTuftsCell::TallWildPatch),
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
fn patches_outweigh_single_clumps() -> Result<()> {
	let placed_weight = |patch: bool| -> f32 {
		CommonTuftsCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item
					.is_some_and(|cell| matches!(cell.item(), CommonTuftsItem::Patch(_)) == patch)
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
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = CommonTuftsCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.10..=0.35).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn clump_geometry_follows_authored_bands() -> Result<()> {
	for cell in [CommonTuftsCell::ShortGreen, CommonTuftsCell::DryScrub, CommonTuftsCell::TallWild]
	{
		let CommonTuftsItem::Clump(clump) = cell.item() else {
			anyhow::bail!("expected clump item for {cell:?}");
		};
		assert!(clump.height.start >= 0.10);
		assert!(clump.height.end <= 1.0);
		assert!(clump.width_factor.start > 0.0);
		assert!(clump.width_factor.end <= 0.05, "blades should stay grass-thin");
	}
	Ok(())
}

#[test]
fn patch_wraps_short_green_clump() -> Result<()> {
	let CommonTuftsItem::Patch(patch) = CommonTuftsCell::ShortGreenPatch.item() else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, SHORT_GREEN);
	assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
	assert!(patch.patch_extent_xz.start > 0.0);
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	// Match the frontend default: cellular per-cell hash values for placement draws.
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let placements = grove.populate(&extent, &terrain);
	assert!(!placements.is_empty());

	// With ±cell offsets, a healthy share of placements should sit far from any cell
	// center; near-center clustering is what reads as a grid.
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
		GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0)),
		&FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
	)
}
