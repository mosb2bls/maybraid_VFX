use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = OrchardCell::distribution();
	assert_eq!(dist.len(), 3);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, CULTIVATED_EMPTY_WEIGHT);
	assert_eq!(dist.buckets[1].item, Some(OrchardCell::FruitingStorybook));
	assert_eq!(dist.buckets[1].weight, 1.5);
	assert_eq!(dist.buckets[2].item, Some(OrchardCell::PaleBloomStorybook));
	assert_eq!(dist.buckets[2].weight, 0.75);
	Ok(())
}

#[test]
fn placed_share_targets_cultivated_fill() -> Result<()> {
	let dist = OrchardCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.94..=0.96).contains(&share), "placed share {share} outside cultivated ~95% target");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let OrchardItem::Storybook(fruiting) = OrchardCell::FruitingStorybook.item();
	assert_eq!(fruiting.height, UnitRange::new(5.0, 10.0));
	assert_eq!(fruiting.canopy_density, MODERATE_CANOPY_DENSITY);

	let OrchardItem::Storybook(pale) = OrchardCell::PaleBloomStorybook.item();
	assert_eq!(pale.height, UnitRange::new(5.0, 9.0));
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = OrchardCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let fruiting = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(OrchardCell::FruitingStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing fruiting bucket"))?;
	assert_eq!(fruiting.constraints.steepness.end, 0.30);

	let pale = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(OrchardCell::PaleBloomStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing pale bloom bucket"))?;
	assert_eq!(pale.constraints.steepness.end, 0.28);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [OrchardCell::FruitingStorybook, OrchardCell::PaleBloomStorybook] {
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
fn placement_uses_tight_centroid_offset_and_uniform_scale() -> Result<()> {
	let def = definition();
	assert_eq!(def.placement.offset, UnitRange::new(-0.5, 0.5));
	assert_eq!(def.placement.scale, UnitRange::new(1.0, 1.0));
	Ok(())
}

#[test]
fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(120.0, 1.0, 120.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.10 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
