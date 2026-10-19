use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = RollingOaksCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(RollingOaksCell::RollingBraidOak),
			Some(RollingOaksCell::RareTallRollingBraidOak),
			Some(RollingOaksCell::RareSentinelRollingBraidOak),
			Some(RollingOaksCell::RareRollingStorybook),
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
	let dist = RollingOaksCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let RollingOaksItem::BraidOak(oak) = RollingOaksCell::RollingBraidOak.item() else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(5.0, 20.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let RollingOaksItem::BraidOak(tall) = RollingOaksCell::RareTallRollingBraidOak.item() else {
		anyhow::bail!("expected rare tall braid oak item");
	};
	assert_eq!(tall.height, UnitRange::new(20.0, 32.0));

	let RollingOaksItem::BraidOak(sentinel) = RollingOaksCell::RareSentinelRollingBraidOak.item()
	else {
		anyhow::bail!("expected rare sentinel braid oak item");
	};
	assert_eq!(sentinel.height, UnitRange::new(28.0, 40.0));

	let RollingOaksItem::Storybook(story) = RollingOaksCell::RareRollingStorybook.item() else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(5.0, 20.0));
	assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = RollingOaksCell::distribution();
	let braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RollingOaksCell::RollingBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
	assert_eq!(braid_oak.constraints.elevation.start, 0.0);
	assert_eq!(braid_oak.constraints.elevation.end, 1.0);
	assert_eq!(braid_oak.constraints.steepness.end, 0.48);

	let tall_braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RollingOaksCell::RareTallRollingBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing tall braid oak bucket"))?;
	assert_eq!(tall_braid_oak.constraints.elevation.end, 1.0);
	assert_eq!(tall_braid_oak.constraints.steepness.end, 0.48);

	let sentinel_braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RollingOaksCell::RareSentinelRollingBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing sentinel braid oak bucket"))?;
	assert_eq!(sentinel_braid_oak.constraints.elevation.end, 1.0);
	assert_eq!(sentinel_braid_oak.constraints.steepness.end, 0.44);

	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RollingOaksCell::RareRollingStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.start, 0.0);
	assert_eq!(storybook.constraints.elevation.end, 1.0);
	assert_eq!(storybook.constraints.steepness.end, 0.54);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		RollingOaksCell::RollingBraidOak,
		RollingOaksCell::RareTallRollingBraidOak,
		RollingOaksCell::RareSentinelRollingBraidOak,
		RollingOaksCell::RareRollingStorybook,
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
		&FlatTerrainSample { elevation: 0.40, steepness: 0.15 },
	)
}
