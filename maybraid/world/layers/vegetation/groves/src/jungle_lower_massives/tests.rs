use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = JungleLowerMassivesCell::distribution();
	assert_eq!(dist.len(), 6);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 8.0);
	assert_eq!(dist.buckets[1].item, Some(JungleLowerMassivesCell::LowerMassiveJungleStorybook));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan));
	assert_eq!(dist.buckets[2].weight, 2.0);
	assert_eq!(dist.buckets[3].item, Some(JungleLowerMassivesCell::LowerMassiveSopesBanyan));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm));
	assert_eq!(dist.buckets[4].weight, 1.0);
	assert_eq!(dist.buckets[5].item, Some(JungleLowerMassivesCell::RareLowerMassiveBraidOak));
	assert_eq!(dist.buckets[5].weight, 0.35);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = JungleLowerMassivesCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.45).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let JungleLowerMassivesItem::JungleStorybook(jungle) =
		JungleLowerMassivesCell::LowerMassiveJungleStorybook.item()
	else {
		anyhow::bail!("expected jungle storybook item");
	};
	assert_eq!(jungle.height, UnitRange::new(10.0, 20.0));

	let JungleLowerMassivesItem::Honu(honu) =
		JungleLowerMassivesCell::LowerMassiveHonuBanyan.item()
	else {
		anyhow::bail!("expected honu item");
	};
	assert_eq!(honu.height, UnitRange::new(10.0, 20.0));
	assert_eq!(honu.canopy_density, DENSE_CANOPY_DENSITY);

	let JungleLowerMassivesItem::BraidOak(oak) =
		JungleLowerMassivesCell::RareLowerMassiveBraidOak.item()
	else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn waialea_accepts_steeper_slope_than_honu() -> Result<()> {
	let dist = JungleLowerMassivesCell::distribution();
	let honu = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan))
		.ok_or_else(|| anyhow::anyhow!("missing honu bucket"))?;
	let waialea = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm))
		.ok_or_else(|| anyhow::anyhow!("missing waialea bucket"))?;
	assert!(waialea.constraints.steepness.end > honu.constraints.steepness.end);
	assert_eq!(honu.constraints.steepness.end, 0.46);
	assert_eq!(waialea.constraints.steepness.end, 0.62);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_honu_but_allows_waialea() -> Result<()> {
	let prepared = JungleLowerMassivesCell::distribution().prepare(
		0.0,
		0.0,
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
	let outcome = prepared.select_from(
		8,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, JungleLowerMassivesCell::LowerMassiveHonuBanyan);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		JungleLowerMassivesCell::LowerMassiveJungleStorybook,
		JungleLowerMassivesCell::LowerMassiveHonuBanyan,
		JungleLowerMassivesCell::LowerMassiveSopesBanyan,
		JungleLowerMassivesCell::LowerMassiveWaialeaPalm,
		JungleLowerMassivesCell::RareLowerMassiveBraidOak,
	] {
		for (palette, label) in
			[(cell.stick_palette_mix(), "stick"), (cell.canopy_palette_mix(), "canopy")]
		{
			let mut allowed = Vec::new();
			for slot in palette.slots {
				allowed.extend(slot.start.resolve());
				allowed.extend(slot.end.resolve());
			}
			assert!(!allowed.is_empty(), "unresolved {label} tokens for {cell:?}");
		}
	}
	Ok(())
}

#[test]
fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
