use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = ForlornSavannaCell::distribution();
	assert_eq!(dist.len(), 4);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 30.0);
	assert_eq!(dist.buckets[1].item, Some(ForlornSavannaCell::SavannaRory));
	assert_eq!(dist.buckets[1].weight, 3.0);
	assert_eq!(dist.buckets[2].item, Some(ForlornSavannaCell::AcaciaHighBush));
	assert_eq!(dist.buckets[2].weight, 2.0);
	assert_eq!(dist.buckets[3].item, Some(ForlornSavannaCell::RareSavannaStorybook));
	assert_eq!(dist.buckets[3].weight, 0.2);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = ForlornSavannaCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let ForlornSavannaItem::Rory(rory) = ForlornSavannaCell::SavannaRory.item() else {
		anyhow::bail!("expected rory item");
	};
	assert_eq!(rory.height, UnitRange::new(5.0, 30.0));
	assert_eq!(rory.stalk_radius, UnitRange::new(0.15, 0.20));
	assert_eq!(rory.canopy_spread, UnitRange::new(3.0, 12.0));
	assert_eq!(rory.canopy_density, SPARSE_CANOPY_DENSITY);

	let ForlornSavannaItem::HighBush(bush) = ForlornSavannaCell::AcaciaHighBush.item() else {
		anyhow::bail!("expected high bush item");
	};
	assert_eq!(bush.height, UnitRange::new(5.0, 10.0));

	let ForlornSavannaItem::Storybook(story) = ForlornSavannaCell::RareSavannaStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(10.0, 20.0));
	assert_eq!(story.canopy_density, SPARSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = ForlornSavannaCell::distribution();
	let rory = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ForlornSavannaCell::SavannaRory))
		.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
	assert_eq!(rory.constraints.elevation.start, 0.0);
	assert_eq!(rory.constraints.elevation.end, 1.0);
	assert_eq!(rory.constraints.steepness.end, 0.58);

	let high_bush = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ForlornSavannaCell::AcaciaHighBush))
		.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
	assert_eq!(high_bush.constraints.elevation.end, 1.0);
	assert_eq!(high_bush.constraints.steepness.end, 0.64);

	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ForlornSavannaCell::RareSavannaStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.start, 0.0);
	assert_eq!(storybook.constraints.steepness.end, 0.50);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_rory_but_allows_high_bush() -> Result<()> {
	let prepared =
		ForlornSavannaCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
	let bush_outcome = prepared.select_from(
		5,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match bush_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, ForlornSavannaCell::AcaciaHighBush);
		}
		other => anyhow::bail!("expected AcaciaHighBush on moderate slope, got {other:?}"),
	}
	let rory_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match rory_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, ForlornSavannaCell::SavannaRory);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		ForlornSavannaCell::SavannaRory,
		ForlornSavannaCell::AcaciaHighBush,
		ForlornSavannaCell::RareSavannaStorybook,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0));
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.20 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
