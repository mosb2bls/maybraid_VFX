use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = ChristmasTaigaCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(ChristmasTaigaCell::ChristmasNorthernConifer),
			Some(ChristmasTaigaCell::HighBandNorthernConifer),
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
	crate::grove::populate_checks::assert_populated_grove_is_deterministic_and_non_empty(
		definition(),
		crate::grove::GroveFrontend::default().noise,
		GroveExtent::new(Vec3::ZERO, Vec3::new(200.0, 1.0, 200.0)),
		&FlatTerrainSample::default(),
	)
}
