use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = ChristmasTaigaCell::distribution();
	assert_eq!(dist.len(), 3);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 3.3);
	assert_eq!(dist.buckets[1].item, Some(ChristmasTaigaCell::ChristmasNorthernConifer));
	assert_eq!(dist.buckets[1].weight, 1.0);
	assert_eq!(dist.buckets[2].item, Some(ChristmasTaigaCell::HighBandNorthernConifer));
	assert_eq!(dist.buckets[2].weight, 0.5);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = ChristmasTaigaCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.20..=0.42).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let ChristmasTaigaItem::NorthernConifer(christmas) =
		ChristmasTaigaCell::ChristmasNorthernConifer.item();
	assert_eq!(christmas.height, UnitRange::new(8.0, 20.0));
	assert_eq!(christmas.canopy_density, DENSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in
		[ChristmasTaigaCell::ChristmasNorthernConifer, ChristmasTaigaCell::HighBandNorthernConifer]
	{
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0));
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
