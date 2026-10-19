use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = LeewardCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(LeewardCell::ShelteredTemperateConifer),
			Some(LeewardCell::WindbreakTemperateConifer),
			Some(LeewardCell::RoundedLeewardStorybook),
			Some(LeewardCell::HighLeewardStorybook),
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
	let dist = LeewardCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.61).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let LeewardItem::TemperateConifer(sheltered) = LeewardCell::ShelteredTemperateConifer.item()
	else {
		anyhow::bail!("expected sheltered temperate conifer item");
	};
	assert_eq!(sheltered.height, UnitRange::new(10.0, 18.0));
	assert_eq!(sheltered.canopy_density, MODERATE_CANOPY_DENSITY);

	let LeewardItem::TemperateConifer(windbreak) = LeewardCell::WindbreakTemperateConifer.item()
	else {
		anyhow::bail!("expected windbreak temperate conifer item");
	};
	assert_eq!(windbreak.height, UnitRange::new(16.0, 24.0));
	assert_eq!(windbreak.canopy_density, SPARSE_CANOPY_DENSITY);

	let LeewardItem::Storybook(rounded) = LeewardCell::RoundedLeewardStorybook.item() else {
		anyhow::bail!("expected rounded leeward storybook item");
	};
	assert_eq!(rounded.height, UnitRange::new(10.0, 18.0));
	assert_eq!(rounded.canopy_density, DENSE_CANOPY_DENSITY);

	let LeewardItem::Storybook(high) = LeewardCell::HighLeewardStorybook.item() else {
		anyhow::bail!("expected high leeward storybook item");
	};
	assert_eq!(high.height, UnitRange::new(16.0, 24.0));
	assert_eq!(high.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = LeewardCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let sheltered = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(LeewardCell::ShelteredTemperateConifer))
		.ok_or_else(|| anyhow::anyhow!("missing sheltered temperate bucket"))?;
	assert_eq!(sheltered.constraints.steepness.end, 0.50);

	let windbreak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(LeewardCell::WindbreakTemperateConifer))
		.ok_or_else(|| anyhow::anyhow!("missing windbreak temperate bucket"))?;
	assert_eq!(windbreak.constraints.steepness.end, 0.66);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		LeewardCell::ShelteredTemperateConifer,
		LeewardCell::WindbreakTemperateConifer,
		LeewardCell::RoundedLeewardStorybook,
		LeewardCell::HighLeewardStorybook,
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
