use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = ConiferMassivesCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 23.0);
	assert_eq!(dist.buckets[1].item, Some(ConiferMassivesCell::MassiveNorthernConifer));
	assert_eq!(dist.buckets[1].weight, 1.25);
	assert_eq!(dist.buckets[2].item, Some(ConiferMassivesCell::MassiveFriendsConifer));
	assert_eq!(dist.buckets[2].weight, 1.25);
	assert_eq!(dist.buckets[3].item, Some(ConiferMassivesCell::MassiveLiamsConifer));
	assert_eq!(dist.buckets[3].weight, 0.75);
	assert_eq!(dist.buckets[4].item, Some(ConiferMassivesCell::MassiveTemperateConifer));
	assert_eq!(dist.buckets[4].weight, 0.25);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = ConiferMassivesCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.06..=0.20).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let ConiferMassivesItem::NorthernConifer(northern) =
		ConiferMassivesCell::MassiveNorthernConifer.item()
	else {
		anyhow::bail!("expected northern conifer item");
	};
	assert_eq!(northern.height, UnitRange::new(70.0, 200.0));
	assert_eq!(northern.canopy_density, DENSE_CANOPY_DENSITY);

	let ConiferMassivesItem::FriendsConifer(friends) =
		ConiferMassivesCell::MassiveFriendsConifer.item()
	else {
		anyhow::bail!("expected friends conifer item");
	};
	assert_eq!(friends.height, UnitRange::new(100.0, 130.0));

	let ConiferMassivesItem::LiamsConifer(liams) = ConiferMassivesCell::MassiveLiamsConifer.item()
	else {
		anyhow::bail!("expected liams conifer item");
	};
	assert_eq!(liams.height, UnitRange::new(25.0, 130.0));
	assert_eq!(liams.canopy_density, MODERATE_CANOPY_DENSITY);

	let ConiferMassivesItem::TemperateConifer(temperate) =
		ConiferMassivesCell::MassiveTemperateConifer.item()
	else {
		anyhow::bail!("expected temperate conifer item");
	};
	assert_eq!(temperate.height, UnitRange::new(40.0, 120.0));
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = ConiferMassivesCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let northern = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ConiferMassivesCell::MassiveNorthernConifer))
		.ok_or_else(|| anyhow::anyhow!("missing northern bucket"))?;
	assert_eq!(northern.constraints.steepness.end, 0.70);

	let friends = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ConiferMassivesCell::MassiveFriendsConifer))
		.ok_or_else(|| anyhow::anyhow!("missing friends bucket"))?;
	assert_eq!(friends.constraints.steepness.end, 0.64);

	let temperate = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(ConiferMassivesCell::MassiveTemperateConifer))
		.ok_or_else(|| anyhow::anyhow!("missing temperate bucket"))?;
	assert_eq!(temperate.constraints.steepness.end, 0.58);
	Ok(())
}

#[test]
fn steep_slope_rejects_friends_but_allows_liams() -> Result<()> {
	let prepared =
		ConiferMassivesCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.40, steepness: 0.68 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, ConiferMassivesCell::MassiveFriendsConifer);
			assert_ne!(variant, ConiferMassivesCell::MassiveNorthernConifer);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		ConiferMassivesCell::MassiveNorthernConifer,
		ConiferMassivesCell::MassiveFriendsConifer,
		ConiferMassivesCell::MassiveLiamsConifer,
		ConiferMassivesCell::MassiveTemperateConifer,
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
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
