use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = BraidGrassCell::distribution();
	assert_eq!(dist.len(), 10);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 2.5);
	assert_eq!(dist.buckets[1].item, Some(BraidGrassCell::DeepGreenBlade));
	assert_eq!(dist.buckets[1].weight, 0.5);
	assert_eq!(dist.buckets[2].weight, 0.25);
	assert_eq!(dist.buckets[3].weight, 0.25);
	assert_eq!(dist.buckets[4].item, Some(BraidGrassCell::RedEdgeBlade));
	assert_eq!(dist.buckets[4].weight, 0.25);
	assert_eq!(dist.buckets[5].item, Some(BraidGrassCell::GreenSpear));
	assert_eq!(dist.buckets[5].weight, 1.0);
	assert_eq!(dist.buckets[6].item, Some(BraidGrassCell::FountainSpear));
	assert_eq!(dist.buckets[6].weight, 0.75);
	assert_eq!(dist.buckets[7].item, Some(BraidGrassCell::DeepGreenPatch));
	assert_eq!(dist.buckets[7].weight, 2.0);
	assert_eq!(dist.buckets[8].item, Some(BraidGrassCell::PaleReedPatch));
	assert_eq!(dist.buckets[8].weight, 1.0);
	assert_eq!(dist.buckets[9].item, Some(BraidGrassCell::JunglePatch));
	assert_eq!(dist.buckets[9].weight, 1.0);
	Ok(())
}

#[test]
fn patches_outweigh_single_blade_clumps() -> Result<()> {
	let blade_weight = |patch: bool| -> f32 {
		BraidGrassCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match cell.item() {
					BraidGrassItem::Blade(_) => !patch,
					BraidGrassItem::Patch(_) => patch,
					BraidGrassItem::Spear(_) => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(blade_weight(true) > 2.0 * blade_weight(false), "patches should dominate blade weight");
	Ok(())
}

#[test]
fn tilt_bands_mix_moderate_and_wide() -> Result<()> {
	// Most varietals stay in a moderate tilt regime; Jungle and FountainSpear take wide
	// bands so their individual clumps span upright through fully splayed.
	let tilt = |cell: BraidGrassCell| match cell.item() {
		BraidGrassItem::Blade(clump) => clump.max_tilt_radians,
		BraidGrassItem::Spear(clump) => clump.max_tilt_radians,
		BraidGrassItem::Patch(patch) => patch.clump.max_tilt_radians,
	};
	for moderate in [
		BraidGrassCell::DeepGreenBlade,
		BraidGrassCell::PaleReedBlade,
		BraidGrassCell::RedEdgeBlade,
		BraidGrassCell::GreenSpear,
	] {
		let band = tilt(moderate);
		assert!(band.start >= 0.05, "{moderate:?} should not be extreme-upright");
		assert!(band.end <= 0.60, "{moderate:?} should not be extreme-splayed");
	}
	for wide in [BraidGrassCell::JungleBlade, BraidGrassCell::FountainSpear] {
		let band = tilt(wide);
		assert!(band.end - band.start >= 0.6, "{wide:?} should span a wide tilt band");
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// Jungle (index 3) rejects steepness 0.35; first-fit wraps to RedEdge (index 4).
	let prepared =
		BraidGrassCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.3, steepness: 0.35 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.3, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, BraidGrassCell::RedEdgeBlade);
		}
		other => anyhow::bail!("expected RedEdgeBlade fallback, got {other:?}"),
	}
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
