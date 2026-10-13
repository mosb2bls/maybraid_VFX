use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = CommonTuftsCell::distribution();
	assert_eq!(dist.len(), 7);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 13.78);
	assert_eq!(dist.buckets[1].item, Some(CommonTuftsCell::ShortGreen));
	assert_eq!(dist.buckets[1].weight, 0.5);
	assert_eq!(dist.buckets[2].item, Some(CommonTuftsCell::DryScrub));
	assert_eq!(dist.buckets[2].weight, 0.25);
	assert_eq!(dist.buckets[3].item, Some(CommonTuftsCell::TallWild));
	assert_eq!(dist.buckets[3].weight, 0.25);
	assert_eq!(dist.buckets[4].item, Some(CommonTuftsCell::ShortGreenPatch));
	assert_eq!(dist.buckets[4].weight, 2.0);
	assert_eq!(dist.buckets[5].item, Some(CommonTuftsCell::DryScrubPatch));
	assert_eq!(dist.buckets[5].weight, 1.0);
	assert_eq!(dist.buckets[6].item, Some(CommonTuftsCell::TallWildPatch));
	assert_eq!(dist.buckets[6].weight, 1.0);
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
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// ShortGreen (index 1) rejects elevation 0.85; first-fit falls to DryScrub (index 2).
	let prepared =
		CommonTuftsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.85, steepness: 0.2 };
	let outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.85, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, CommonTuftsCell::DryScrub);
		}
		other => anyhow::bail!("expected DryScrub fallback, got {other:?}"),
	}
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
