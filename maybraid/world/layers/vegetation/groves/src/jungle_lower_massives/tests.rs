use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = JungleLowerMassivesCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(JungleLowerMassivesCell::LowerMassiveJungleStorybook),
			Some(JungleLowerMassivesCell::LowerMassiveHonuBanyan),
			Some(JungleLowerMassivesCell::LowerMassiveSopesBanyan),
			Some(JungleLowerMassivesCell::LowerMassiveWaialeaPalm),
			Some(JungleLowerMassivesCell::RareLowerMassiveBraidOak),
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
