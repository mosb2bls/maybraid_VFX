use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = RiparianGeneralCell::distribution();
	assert_eq!(dist.len(), 4);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 7.4);
	assert_eq!(dist.buckets[1].item, Some(RiparianGeneralCell::RiparianBraidOak));
	assert_eq!(dist.buckets[1].weight, 1.5);
	assert_eq!(dist.buckets[2].item, Some(RiparianGeneralCell::RiparianStorybook));
	assert_eq!(dist.buckets[2].weight, 1.5);
	assert_eq!(dist.buckets[3].item, Some(RiparianGeneralCell::RareRiparianHighBush));
	assert_eq!(dist.buckets[3].weight, 0.35);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = RiparianGeneralCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.20..=0.42).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let RiparianGeneralItem::BraidOak(oak) = RiparianGeneralCell::RiparianBraidOak.item() else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(5.0, 15.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianGeneralItem::Storybook(story) = RiparianGeneralCell::RiparianStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(5.0, 15.0));
	assert_eq!(story.canopy_density, MODERATE_CANOPY_DENSITY);

	let RiparianGeneralItem::HighBush(bush) = RiparianGeneralCell::RareRiparianHighBush.item()
	else {
		anyhow::bail!("expected high bush item");
	};
	assert_eq!(bush.height, UnitRange::new(5.0, 15.0));
	assert_eq!(bush.leaf_radius, UnitRange::new(0.12, 0.28));
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = RiparianGeneralCell::distribution();
	let braid_oak = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RiparianBraidOak))
		.ok_or_else(|| anyhow::anyhow!("missing braid oak bucket"))?;
	assert_eq!(braid_oak.constraints.elevation.start, 0.0);
	assert_eq!(braid_oak.constraints.elevation.end, 1.0);
	assert_eq!(braid_oak.constraints.steepness.end, 0.36);

	let storybook = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RiparianStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing storybook bucket"))?;
	assert_eq!(storybook.constraints.elevation.start, 0.0);
	assert_eq!(storybook.constraints.elevation.end, 1.0);
	assert_eq!(storybook.constraints.steepness.end, 0.44);

	let high_bush = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(RiparianGeneralCell::RareRiparianHighBush))
		.ok_or_else(|| anyhow::anyhow!("missing high bush bucket"))?;
	assert_eq!(high_bush.constraints.elevation.start, 0.0);
	assert_eq!(high_bush.constraints.elevation.end, 1.0);
	assert_eq!(high_bush.constraints.steepness.end, 0.52);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_braid_oak_but_allows_high_bush() -> Result<()> {
	let prepared =
		RiparianGeneralCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.45 };
	let bush_outcome = prepared.select_from(
		5,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match bush_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, RiparianGeneralCell::RareRiparianHighBush);
		}
		other => {
			anyhow::bail!("expected RareRiparianHighBush on moderate slope, got {other:?}")
		}
	}
	let braid_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match braid_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, RiparianGeneralCell::RiparianBraidOak);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		RiparianGeneralCell::RiparianBraidOak,
		RiparianGeneralCell::RiparianStorybook,
		RiparianGeneralCell::RareRiparianHighBush,
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(160.0, 1.0, 160.0));
	let terrain = FlatTerrainSample { elevation: 0.20, steepness: 0.10 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
