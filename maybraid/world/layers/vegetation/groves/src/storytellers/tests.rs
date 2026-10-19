use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_authored_order_and_membership() -> Result<()> {
	let dist = StorytellersCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(StorytellersCell::ColorfulStorybook),
			Some(StorytellersCell::ColorfulBraidOak),
			Some(StorytellersCell::BrightCanopyStorybook),
			Some(StorytellersCell::PinkLanternStorybook),
			Some(StorytellersCell::RedFestivalBraidOak),
			Some(StorytellersCell::PurpleCrownStorybook),
			Some(StorytellersCell::BlueMoonStorybook),
			Some(StorytellersCell::GoldenLanternPenmarch),
			Some(StorytellersCell::BlueFlameKamakura),
			Some(StorytellersCell::FestivalTorchTree),
			Some(StorytellersCell::VioletCanopyBraidOak),
			Some(StorytellersCell::GoldLeafBraidOak),
			Some(StorytellersCell::CopperFlameBraidOak),
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
	let dist = StorytellersCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let StorytellersItem::Storybook(colorful) = StorytellersCell::ColorfulStorybook.item() else {
		anyhow::bail!("expected colorful storybook item");
	};
	assert_eq!(colorful.height, UnitRange::new(10.0, 30.0));
	assert_eq!(colorful.canopy_density, DENSE_CANOPY_DENSITY);

	let StorytellersItem::BraidOak(festival) = StorytellersCell::RedFestivalBraidOak.item() else {
		anyhow::bail!("expected red festival braid oak item");
	};
	assert_eq!(festival.height, UnitRange::new(12.0, 24.0));
	assert_eq!(festival.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = StorytellersCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let braid = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(StorytellersCell::ColorfulBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing colorful braid oak bucket"))?;
	assert_eq!(braid.constraints.steepness.end, 0.48);

	let bright = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(StorytellersCell::BrightCanopyStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing bright storybook bucket"))?;
	assert_eq!(bright.constraints.steepness.end, 0.56);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		StorytellersCell::ColorfulStorybook,
		StorytellersCell::ColorfulBraidOak,
		StorytellersCell::BrightCanopyStorybook,
		StorytellersCell::PinkLanternStorybook,
		StorytellersCell::RedFestivalBraidOak,
		StorytellersCell::PurpleCrownStorybook,
		StorytellersCell::BlueMoonStorybook,
		StorytellersCell::GoldenLanternPenmarch,
		StorytellersCell::BlueFlameKamakura,
		StorytellersCell::FestivalTorchTree,
		StorytellersCell::VioletCanopyBraidOak,
		StorytellersCell::GoldLeafBraidOak,
		StorytellersCell::CopperFlameBraidOak,
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
	crate::grove::populate_checks::assert_populated_grove_is_deterministic_and_non_empty(
		definition(),
		crate::grove::GroveFrontend::default().noise,
		GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0)),
		&FlatTerrainSample::default(),
	)
}
