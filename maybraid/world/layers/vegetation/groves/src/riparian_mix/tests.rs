use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = RiparianMixCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(RiparianMixCell::BankBraidOak),
			Some(RiparianMixCell::OverbankBraidOak),
			Some(RiparianMixCell::RoundRiparianStorybook),
			Some(RiparianMixCell::TallRiparianStorybook),
			Some(RiparianMixCell::BankFriendConifer),
			Some(RiparianMixCell::ShelteredTemperateConifer),
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
	let dist = RiparianMixCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.40).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let RiparianMixItem::BraidOak(bank) = RiparianMixCell::BankBraidOak.item() else {
		anyhow::bail!("expected bank braid oak item");
	};
	assert_eq!(bank.height, UnitRange::new(5.0, 12.0));
	assert_eq!(bank.canopy_density, DENSE_CANOPY_DENSITY);

	let RiparianMixItem::BraidOak(overbank) = RiparianMixCell::OverbankBraidOak.item() else {
		anyhow::bail!("expected overbank braid oak item");
	};
	assert_eq!(overbank.height, UnitRange::new(10.0, 18.0));
	assert_eq!(overbank.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianMixItem::Storybook(round) = RiparianMixCell::RoundRiparianStorybook.item() else {
		anyhow::bail!("expected round storybook item");
	};
	assert_eq!(round.height, UnitRange::new(5.0, 15.0));
	assert_eq!(round.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianMixItem::Storybook(tall) = RiparianMixCell::TallRiparianStorybook.item() else {
		anyhow::bail!("expected tall storybook item");
	};
	assert_eq!(tall.height, UnitRange::new(12.0, 22.0));
	assert_eq!(tall.canopy_density, SPARSE_CANOPY_DENSITY);

	let RiparianMixItem::FriendsConifer(friend) = RiparianMixCell::BankFriendConifer.item() else {
		anyhow::bail!("expected friend conifer item");
	};
	assert_eq!(friend.height, UnitRange::new(8.0, 16.0));
	assert_eq!(friend.canopy_density, DENSE_CANOPY_DENSITY);

	let RiparianMixItem::TemperateConifer(temperate) =
		RiparianMixCell::ShelteredTemperateConifer.item()
	else {
		anyhow::bail!("expected temperate conifer item");
	};
	assert_eq!(temperate.height, UnitRange::new(10.0, 20.0));
	assert_eq!(temperate.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = RiparianMixCell::distribution();
	let bank = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianMixCell::BankBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing bank braid oak bucket"))?;
	assert_eq!(bank.constraints.elevation.end, 0.38);
	assert_eq!(bank.constraints.steepness.end, 0.30);

	let overbank = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianMixCell::OverbankBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing overbank braid oak bucket"))?;
	assert_eq!(overbank.constraints.elevation.end, 0.48);
	assert_eq!(overbank.constraints.steepness.end, 0.42);

	let friend = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianMixCell::BankFriendConifer))
		.ok_or_else(|| anyhow::anyhow!("missing bank friend conifer bucket"))?;
	assert_eq!(friend.constraints.elevation.end, 0.58);
	assert_eq!(friend.constraints.steepness.end, 0.50);

	let temperate = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianMixCell::ShelteredTemperateConifer))
		.ok_or_else(|| anyhow::anyhow!("missing sheltered temperate conifer bucket"))?;
	assert_eq!(temperate.constraints.elevation.end, 0.62);
	assert_eq!(temperate.constraints.steepness.end, 0.54);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		RiparianMixCell::BankBraidOak,
		RiparianMixCell::OverbankBraidOak,
		RiparianMixCell::RoundRiparianStorybook,
		RiparianMixCell::TallRiparianStorybook,
		RiparianMixCell::BankFriendConifer,
		RiparianMixCell::ShelteredTemperateConifer,
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
		GroveExtent::new(Vec3::ZERO, Vec3::new(180.0, 1.0, 180.0)),
		&FlatTerrainSample::default(),
	)
}
