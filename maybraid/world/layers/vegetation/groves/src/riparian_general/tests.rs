use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = RiparianGeneralCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(RiparianGeneralCell::RiparianBraidOak),
			Some(RiparianGeneralCell::RiparianStorybook),
			Some(RiparianGeneralCell::RareRiparianHighBush),
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
	let dist = RiparianGeneralCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.20..=0.42).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let RiparianGeneralItem::BraidOak(oak) = RiparianGeneralCell::RiparianBraidOak.item() else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(5.0, 15.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianGeneralItem::Storybook(story) = RiparianGeneralCell::RiparianStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(5.0, 15.0));
	assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianGeneralItem::HighBush(bush) = RiparianGeneralCell::RareRiparianHighBush.item()
	else {
		anyhow::bail!("expected high bush item");
	};
	assert_eq!(bush.height, UnitRange::new(5.0, 15.0));
	assert_eq!(bush.leaf_radius, UnitRange::new(0.12, 0.28));
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = RiparianGeneralCell::distribution();
	let braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RiparianBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
	assert_eq!(braid_oak.constraints.elevation.start, 0.0);
	assert_eq!(braid_oak.constraints.elevation.end, 1.0);
	assert_eq!(braid_oak.constraints.steepness.end, 0.36);

	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RiparianStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.start, 0.0);
	assert_eq!(storybook.constraints.elevation.end, 1.0);
	assert_eq!(storybook.constraints.steepness.end, 0.44);

	let high_bush = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RareRiparianHighBush))
		.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
	assert_eq!(high_bush.constraints.elevation.start, 0.0);
	assert_eq!(high_bush.constraints.elevation.end, 1.0);
	assert_eq!(high_bush.constraints.steepness.end, 0.52);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		RiparianGeneralCell::RiparianBraidOak,
		RiparianGeneralCell::RiparianStorybook,
		RiparianGeneralCell::RareRiparianHighBush,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0));
	let terrain = FlatTerrainSample { elevation: 0.20, steepness: 0.10 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
