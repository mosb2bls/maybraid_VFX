use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_authored_order_and_weights() -> Result<()> {
	let dist = StorytellersCell::distribution();
	assert_eq!(dist.len(), 14);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 16.2);
	assert_eq!(dist.buckets[1].item, Some(StorytellersCell::ColorfulStorybook));
	assert_eq!(dist.buckets[1].weight, 1.5);
	assert_eq!(dist.buckets[2].item, Some(StorytellersCell::ColorfulBraidOak));
	assert_eq!(dist.buckets[2].weight, 1.5);
	assert_eq!(dist.buckets[3].item, Some(StorytellersCell::BrightCanopyStorybook));
	assert_eq!(dist.buckets[3].weight, 0.75);
	assert_eq!(dist.buckets[4].item, Some(StorytellersCell::PinkLanternStorybook));
	assert_eq!(dist.buckets[4].weight, 0.35);
	assert_eq!(dist.buckets[5].item, Some(StorytellersCell::RedFestivalBraidOak));
	assert_eq!(dist.buckets[5].weight, 0.30);
	assert_eq!(dist.buckets[6].item, Some(StorytellersCell::PurpleCrownStorybook));
	assert_eq!(dist.buckets[6].weight, 0.25);
	assert_eq!(dist.buckets[7].item, Some(StorytellersCell::BlueMoonStorybook));
	assert_eq!(dist.buckets[7].weight, 0.25);
	assert_eq!(dist.buckets[8].item, Some(StorytellersCell::GoldenLanternPenmarch));
	assert_eq!(dist.buckets[8].weight, 0.22);
	assert_eq!(dist.buckets[9].item, Some(StorytellersCell::BlueFlameKamakura));
	assert_eq!(dist.buckets[9].weight, 0.20);
	assert_eq!(dist.buckets[10].item, Some(StorytellersCell::FestivalTorchTree));
	assert_eq!(dist.buckets[10].weight, 0.18);
	assert_eq!(dist.buckets[11].item, Some(StorytellersCell::VioletCanopyBraidOak));
	assert_eq!(dist.buckets[11].weight, 0.28);
	assert_eq!(dist.buckets[12].item, Some(StorytellersCell::GoldLeafBraidOak));
	assert_eq!(dist.buckets[12].weight, 0.26);
	assert_eq!(dist.buckets[13].item, Some(StorytellersCell::CopperFlameBraidOak));
	assert_eq!(dist.buckets[13].weight, 0.24);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = StorytellersCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.38).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let StorytellersItem::Storybook(colorful) = StorytellersCell::ColorfulStorybook.item() else {
		anyhow::bail!("expected colorful storybook item");
	};
	assert_eq!(colorful.height, UnitRange::new(10.0, 30.0));
	assert_eq!(colorful.canopy_density, DENSE_CANOPY_DENSITY);

	let StorytellersItem::BraidOak(festival) = StorytellersCell::RedFestivalBraidOak.item() else {
		anyhow::bail!("expected red festival braid oak item");
	};
	assert_eq!(festival.height, UnitRange::new(12.0, 24.0));
	assert_eq!(festival.canopy_density, MODERATE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = StorytellersCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let braid = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(StorytellersCell::ColorfulBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing colorful braid oak bucket"))?;
	assert_eq!(braid.constraints.steepness.end, 0.48);

	let bright = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(StorytellersCell::BrightCanopyStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing bright storybook bucket"))?;
	assert_eq!(bright.constraints.steepness.end, 0.56);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_colorful_braid_oak_but_allows_bright_storybook() -> Result<()> {
	let prepared =
		StorytellersCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.40 };
	let braid_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&moderate,
	);
	match braid_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, StorytellersCell::ColorfulBraidOak);
		}
		other => anyhow::bail!("expected ColorfulBraidOak on moderate slope, got {other:?}"),
	}
	let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.50 };
	let steep_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match steep_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, StorytellersCell::BrightCanopyStorybook);
		}
		other => {
			anyhow::bail!("expected BrightCanopyStorybook on steep slope, got {other:?}")
		}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		StorytellersCell::ColorfulStorybook,
		StorytellersCell::ColorfulBraidOak,
		StorytellersCell::BrightCanopyStorybook,
		StorytellersCell::PinkLanternStorybook,
		StorytellersCell::RedFestivalBraidOak,
		StorytellersCell::PurpleCrownStorybook,
		StorytellersCell::BlueMoonStorybook,
		StorytellersCell::GoldenLanternPenmarch,
		StorytellersCell::BlueFlameKamakura,
		StorytellersCell::FestivalTorchTree,
		StorytellersCell::VioletCanopyBraidOak,
		StorytellersCell::GoldLeafBraidOak,
		StorytellersCell::CopperFlameBraidOak,
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
