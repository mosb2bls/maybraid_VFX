use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = AlpineCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 9.5);
	assert_eq!(dist.buckets[1].item, Some(AlpineCell::TallAlpineFriendsConifer));
	assert_eq!(dist.buckets[1].weight, 1.5);
	assert_eq!(dist.buckets[2].item, Some(AlpineCell::WindlineFriendsConifer));
	assert_eq!(dist.buckets[2].weight, 0.75);
	assert_eq!(dist.buckets[3].item, Some(AlpineCell::AlpineLiamsConifer));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(AlpineCell::NeedleSpireLiamsConifer));
	assert_eq!(dist.buckets[4].weight, 0.45);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = AlpineCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let AlpineItem::FriendsConifer(tall) = AlpineCell::TallAlpineFriendsConifer.item() else {
		anyhow::bail!("expected tall friends item");
	};
	assert_eq!(tall.height, UnitRange::new(12.0, 40.0));
	assert_eq!(tall.canopy_density, DENSE_CANOPY_DENSITY);

	let AlpineItem::LiamsConifer(spire) = AlpineCell::NeedleSpireLiamsConifer.item() else {
		anyhow::bail!("expected needle spire item");
	};
	assert_eq!(spire.height, UnitRange::new(6.0, 32.0));
	assert_eq!(spire.canopy_density, SPARSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = AlpineCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let tall = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(AlpineCell::TallAlpineFriendsConifer))
		.ok_or_else(|| anyhow::anyhow!("missing tall friends bucket"))?;
	assert_eq!(tall.constraints.steepness.end, 0.68);

	let windline = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(AlpineCell::WindlineFriendsConifer))
		.ok_or_else(|| anyhow::anyhow!("missing windline friends bucket"))?;
	assert_eq!(windline.constraints.steepness.end, 0.86);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_tall_friends_but_allows_windline() -> Result<()> {
	let prepared = AlpineCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let moderate = FlatTerrainSample { elevation: 0.30, steepness: 0.40 };
	let moderate_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&moderate,
	);
	match moderate_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, AlpineCell::TallAlpineFriendsConifer);
		}
		other => {
			anyhow::bail!("expected TallAlpineFriendsConifer on moderate slope, got {other:?}")
		}
	}
	let steep = FlatTerrainSample { elevation: 0.30, steepness: 0.70 };
	let steep_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match steep_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, AlpineCell::WindlineFriendsConifer);
		}
		other => anyhow::bail!("expected WindlineFriendsConifer on steep ridge, got {other:?}"),
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		AlpineCell::TallAlpineFriendsConifer,
		AlpineCell::WindlineFriendsConifer,
		AlpineCell::AlpineLiamsConifer,
		AlpineCell::NeedleSpireLiamsConifer,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(220.0, 1.0, 220.0));
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
