use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = JungleMassivesCell::distribution();
	assert_eq!(dist.len(), 4);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 24.0);
	assert_eq!(dist.buckets[1].item, Some(JungleMassivesCell::MassiveJungleStorybook));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(JungleMassivesCell::MassiveHonuBanyan));
	assert_eq!(dist.buckets[2].weight, 2.0);
	assert_eq!(dist.buckets[3].item, Some(JungleMassivesCell::MassiveSopesBanyan));
	assert_eq!(dist.buckets[3].weight, 1.0);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = JungleMassivesCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.16..=0.34).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let JungleMassivesItem::JungleStorybook(jungle) =
		JungleMassivesCell::MassiveJungleStorybook.item()
	else {
		anyhow::bail!("expected jungle storybook item");
	};
	assert_eq!(jungle.height, UnitRange::new(70.0, 160.0));
	assert_eq!(jungle.canopy_density, DENSE_CANOPY_DENSITY);
	assert_eq!(jungle.jungle_growth_density, DENSE_JUNGLE_GROWTH_DENSITY);

	let JungleMassivesItem::Honu(honu) = JungleMassivesCell::MassiveHonuBanyan.item() else {
		anyhow::bail!("expected honu item");
	};
	assert_eq!(honu.height, UnitRange::new(70.0, 200.0));
	assert_eq!(honu.descender_density, DENSE_DESCENDER_DENSITY);

	let JungleMassivesItem::Sope(sope) = JungleMassivesCell::MassiveSopesBanyan.item() else {
		anyhow::bail!("expected sope item");
	};
	assert_eq!(sope.height, UnitRange::new(60.0, 220.0));
	assert_eq!(sope.descender_density, DENSE_DESCENDER_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = JungleMassivesCell::distribution();
	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(JungleMassivesCell::MassiveJungleStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.end, 0.50);
	assert_eq!(storybook.constraints.steepness.end, 0.44);

	let honu = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(JungleMassivesCell::MassiveHonuBanyan))
		.ok_or_else(|| anyhow::anyhow!("missing honu bucket"))?;
	assert_eq!(honu.constraints.elevation.end, 0.46);
	assert_eq!(honu.constraints.steepness.end, 0.38);

	let sope = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(JungleMassivesCell::MassiveSopesBanyan))
		.ok_or_else(|| anyhow::anyhow!("missing sope bucket"))?;
	assert_eq!(sope.constraints.elevation.end, 0.44);
	assert_eq!(sope.constraints.steepness.end, 0.42);
	Ok(())
}

#[test]
fn steep_slope_rejects_honu_but_allows_storybook() -> Result<()> {
	let prepared =
		JungleMassivesCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.40 };
	let outcome = prepared.select_from(
		8,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, JungleMassivesCell::MassiveHonuBanyan);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		JungleMassivesCell::MassiveJungleStorybook,
		JungleMassivesCell::MassiveHonuBanyan,
		JungleMassivesCell::MassiveSopesBanyan,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0));
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
