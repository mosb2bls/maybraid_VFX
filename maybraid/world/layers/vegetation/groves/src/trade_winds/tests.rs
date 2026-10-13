use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = TradeWindsCell::distribution();
	assert_eq!(dist.len(), 6);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 21.5);
	assert_eq!(dist.buckets[1].item, Some(TradeWindsCell::TradeStorybook));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(TradeWindsCell::TradeSopesBanyan));
	assert_eq!(dist.buckets[2].weight, 0.75);
	assert_eq!(dist.buckets[3].item, Some(TradeWindsCell::TradeHonuBanyan));
	assert_eq!(dist.buckets[3].weight, 0.75);
	assert_eq!(dist.buckets[4].item, Some(TradeWindsCell::RareTallTradeStorybook));
	assert_eq!(dist.buckets[4].weight, 0.35);
	assert_eq!(dist.buckets[5].item, Some(TradeWindsCell::RareTradeWaialeaPalm));
	assert_eq!(dist.buckets[5].weight, 0.25);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = TradeWindsCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let TradeWindsItem::Storybook(trade) = TradeWindsCell::TradeStorybook.item() else {
		anyhow::bail!("expected trade storybook item");
	};
	assert_eq!(trade.height, UnitRange::new(10.0, 20.0));
	assert_eq!(trade.canopy_density, MODERATE_CANOPY_DENSITY);

	let TradeWindsItem::Honu(honu) = TradeWindsCell::TradeHonuBanyan.item() else {
		anyhow::bail!("expected trade honu item");
	};
	assert_eq!(honu.height, UnitRange::new(10.0, 25.0));
	assert_eq!(honu.descender_density, SPARSE_DESCENDER_DENSITY);

	let TradeWindsItem::WaialeaPalm(palm) = TradeWindsCell::RareTradeWaialeaPalm.item() else {
		anyhow::bail!("expected rare waialea item");
	};
	assert_eq!(palm.height, UnitRange::new(10.0, 40.0));
	assert_eq!(palm.crown_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = TradeWindsCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let sope = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(TradeWindsCell::TradeSopesBanyan))
		.ok_or_else(|| anyhow::anyhow!("missing trade sope bucket"))?;
	assert_eq!(sope.constraints.steepness.end, 0.44);

	let tall = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(TradeWindsCell::RareTallTradeStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing rare tall storybook bucket"))?;
	assert_eq!(tall.constraints.steepness.end, 0.50);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_trade_sope_but_allows_rare_tall_storybook() -> Result<()> {
	let prepared =
		TradeWindsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.40 };
	let sope_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&moderate,
	);
	match sope_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, TradeWindsCell::TradeSopesBanyan);
		}
		other => anyhow::bail!("expected TradeSopesBanyan on moderate slope, got {other:?}"),
	}
	let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.45 };
	let steep_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match steep_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, TradeWindsCell::RareTallTradeStorybook);
		}
		other => {
			anyhow::bail!("expected RareTallTradeStorybook on steep slope, got {other:?}")
		}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		TradeWindsCell::TradeStorybook,
		TradeWindsCell::TradeSopesBanyan,
		TradeWindsCell::TradeHonuBanyan,
		TradeWindsCell::RareTallTradeStorybook,
		TradeWindsCell::RareTradeWaialeaPalm,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(260.0, 1.0, 260.0));
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
