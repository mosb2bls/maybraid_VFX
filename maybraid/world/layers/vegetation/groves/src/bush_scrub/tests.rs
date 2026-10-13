use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = BushScrubCell::distribution();
	assert_eq!(dist.len(), 7);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 12.0);
	assert_eq!(dist.buckets[1].item, Some(BushScrubCell::DryTuft));
	assert_eq!(dist.buckets[1].weight, 0.4);
	assert_eq!(dist.buckets[2].item, Some(BushScrubCell::GreenTuft));
	assert_eq!(dist.buckets[2].weight, 0.3);
	assert_eq!(dist.buckets[3].item, Some(BushScrubCell::SmallBush));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(BushScrubCell::SaplingBush));
	assert_eq!(dist.buckets[4].weight, 0.5);
	assert_eq!(dist.buckets[5].item, Some(BushScrubCell::DryTuftPatch));
	assert_eq!(dist.buckets[5].weight, 1.6);
	assert_eq!(dist.buckets[6].item, Some(BushScrubCell::GreenTuftPatch));
	assert_eq!(dist.buckets[6].weight, 1.2);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = BushScrubCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.10..=0.30).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn patches_outweigh_single_tufts() -> Result<()> {
	let tuft_weight = |patch: bool| -> f32 {
		BushScrubCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match cell.item() {
					BushScrubItem::Tuft(_) => !patch,
					BushScrubItem::Patch(_) => patch,
					BushScrubItem::Bush(_) => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(tuft_weight(true) > 2.0 * tuft_weight(false), "patches should dominate tuft weight");
	Ok(())
}

#[test]
fn tuft_and_bush_placed_weights_match_rfc_ratio() -> Result<()> {
	let weight = |kind: &str| -> f32 {
		BushScrubCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match (kind, cell.item()) {
					("tuft", BushScrubItem::Tuft(_) | BushScrubItem::Patch(_)) => true,
					("bush", BushScrubItem::Bush(_)) => true,
					_ => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	let tuft = weight("tuft");
	let bush = weight("bush");
	assert!((tuft - 3.5).abs() < 1e-4, "expected tuft weight 3.5, got {tuft}");
	assert!((bush - 1.5).abs() < 1e-4, "expected bush weight 1.5, got {bush}");
	Ok(())
}

#[test]
fn tuft_geometry_follows_authored_bands() -> Result<()> {
	for cell in [BushScrubCell::DryTuft, BushScrubCell::GreenTuft] {
		let BushScrubItem::Tuft(tuft) = cell.item() else {
			anyhow::bail!("expected tuft item for {cell:?}");
		};
		assert!(tuft.height.start >= 0.25);
		assert!(tuft.height.end <= 0.50);
		assert!(tuft.width_factor.start > 0.0);
		assert!(tuft.width_factor.end <= 0.05, "blades should stay grass-thin");
	}
	Ok(())
}

#[test]
fn bush_geometry_follows_authored_bands() -> Result<()> {
	let BushScrubItem::Bush(small) = BushScrubCell::SmallBush.item() else {
		anyhow::bail!("expected small bush item");
	};
	assert!(small.height.start >= 0.35);
	assert!(small.height.end <= 0.80);
	assert_eq!(small.shoot_count, 4..=7);
	assert_eq!(small.branch_depth, 1..=2);

	let BushScrubItem::Bush(sapling) = BushScrubCell::SaplingBush.item() else {
		anyhow::bail!("expected sapling bush item");
	};
	assert!(sapling.height.start >= 0.50);
	assert!(sapling.height.end <= 1.20);
	assert_eq!(sapling.shoot_count, 3..=5);
	assert_eq!(sapling.branch_depth, 1..=1);
	Ok(())
}

#[test]
fn patch_wraps_dry_tuft_clump() -> Result<()> {
	let BushScrubItem::Patch(patch) = BushScrubCell::DryTuftPatch.item() else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, DRY_TUFT);
	assert!(*patch.clump_count.start() >= 2, "a patch should scatter several clumps");
	assert!(patch.patch_extent_xz.start > 0.0);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// GreenTuft (index 2) rejects steepness 0.50; first-fit falls to SmallBush (index 3),
	// which allows steepness up to 0.65.
	let prepared =
		BushScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
	let outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, BushScrubCell::SmallBush);
		}
		other => anyhow::bail!("expected SmallBush fallback, got {other:?}"),
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
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
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
