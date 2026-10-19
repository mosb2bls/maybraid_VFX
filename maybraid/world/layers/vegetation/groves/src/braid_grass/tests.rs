use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = BraidGrassCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(BraidGrassCell::DeepGreenBlade),
			Some(BraidGrassCell::PaleReedBlade),
			Some(BraidGrassCell::JungleBlade),
			Some(BraidGrassCell::RedEdgeBlade),
			Some(BraidGrassCell::GreenSpear),
			Some(BraidGrassCell::FountainSpear),
			Some(BraidGrassCell::DeepGreenPatch),
			Some(BraidGrassCell::PaleReedPatch),
			Some(BraidGrassCell::JunglePatch),
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
