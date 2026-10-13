use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = TemperateLowerMassivesCell::distribution();
	assert_eq!(dist.len(), 4);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 8.0);
	assert_eq!(dist.buckets[1].item, Some(TemperateLowerMassivesCell::LowerMassiveBraidOak));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(TemperateLowerMassivesCell::LowerMassiveStorybook));
	assert_eq!(dist.buckets[2].weight, 2.0);
	assert_eq!(dist.buckets[3].item, Some(TemperateLowerMassivesCell::RareLowerMassiveRory));
	assert_eq!(dist.buckets[3].weight, 0.35);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = TemperateLowerMassivesCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.10..=0.36).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let TemperateLowerMassivesItem::BraidOak(oak) =
		TemperateLowerMassivesCell::LowerMassiveBraidOak.item()
	else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(8.0, 24.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let TemperateLowerMassivesItem::Storybook(story) =
		TemperateLowerMassivesCell::LowerMassiveStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(8.0, 20.0));
	assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);

	let TemperateLowerMassivesItem::Rory(rory) =
		TemperateLowerMassivesCell::RareLowerMassiveRory.item()
	else {
		anyhow::bail!("expected rory item");
	};
	assert_eq!(rory.height, UnitRange::new(10.0, 20.0));
	assert_eq!(rory.canopy_spread, UnitRange::new(2.5, 6.0));
	assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = TemperateLowerMassivesCell::distribution();
	let braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(TemperateLowerMassivesCell::LowerMassiveBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
	assert_eq!(braid_oak.constraints.elevation.start, 0.00);
	assert_eq!(braid_oak.constraints.elevation.end, 0.68);
	assert_eq!(braid_oak.constraints.steepness.end, 0.50);

	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(TemperateLowerMassivesCell::LowerMassiveStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.start, 0.00);
	assert_eq!(storybook.constraints.elevation.end, 0.72);
	assert_eq!(storybook.constraints.steepness.end, 0.56);

	let rory = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(TemperateLowerMassivesCell::RareLowerMassiveRory))
		.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
	assert_eq!(rory.constraints.elevation.start, 0.00);
	assert_eq!(rory.constraints.elevation.end, 0.64);
	assert_eq!(rory.constraints.steepness.end, 0.68);
	Ok(())
}

#[test]
fn steep_slope_rejects_braid_oak_but_allows_rory() -> Result<()> {
	let prepared = TemperateLowerMassivesCell::distribution().prepare(
		0.0,
		0.0,
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.55 };
	let outcome = prepared.select_from(
		8,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, TemperateLowerMassivesCell::LowerMassiveBraidOak);
			assert_ne!(variant, TemperateLowerMassivesCell::LowerMassiveStorybook);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		TemperateLowerMassivesCell::LowerMassiveBraidOak,
		TemperateLowerMassivesCell::LowerMassiveStorybook,
		TemperateLowerMassivesCell::RareLowerMassiveRory,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
