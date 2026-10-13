use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = LeewardCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 4.0);
	assert_eq!(dist.buckets[1].item, Some(LeewardCell::ShelteredTemperateConifer));
	assert_eq!(dist.buckets[1].weight, 1.8);
	assert_eq!(dist.buckets[2].item, Some(LeewardCell::WindbreakTemperateConifer));
	assert_eq!(dist.buckets[2].weight, 1.6);
	assert_eq!(dist.buckets[3].item, Some(LeewardCell::RoundedLeewardStorybook));
	assert_eq!(dist.buckets[3].weight, 2.4);
	assert_eq!(dist.buckets[4].item, Some(LeewardCell::HighLeewardStorybook));
	assert_eq!(dist.buckets[4].weight, 0.45);
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
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_sheltered_conifer_but_falls_through_to_windbreak() -> Result<()> {
	let prepared =
		LeewardCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.45 };
	let sheltered_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&moderate,
	);
	match sheltered_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, LeewardCell::ShelteredTemperateConifer);
		}
		other => {
			anyhow::bail!("expected ShelteredTemperateConifer on moderate slope, got {other:?}")
		}
	}
	let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
	let steep_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match steep_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, LeewardCell::WindbreakTemperateConifer);
		}
		other => {
			anyhow::bail!("expected fall-through to WindbreakTemperateConifer, got {other:?}")
		}
	}
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
