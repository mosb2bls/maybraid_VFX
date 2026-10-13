use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = PalmShadeCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 10.7);
	assert_eq!(dist.buckets[1].item, Some(PalmShadeCell::TowerWaialeaPalm));
	assert_eq!(dist.buckets[1].weight, 0.8);
	assert_eq!(dist.buckets[2].item, Some(PalmShadeCell::LowerWaialeaPalm));
	assert_eq!(dist.buckets[2].weight, 0.8);
	assert_eq!(dist.buckets[3].item, Some(PalmShadeCell::ShadeDatePalm));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(PalmShadeCell::ClusterDatePalm));
	assert_eq!(dist.buckets[4].weight, 0.6);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = PalmShadeCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.08..=0.25).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let PalmShadeItem::WaialeaPalm(tower) = PalmShadeCell::TowerWaialeaPalm.item() else {
		anyhow::bail!("expected tower waialea item");
	};
	assert_eq!(tower.height, UnitRange::new(20.0, 40.0));
	assert_eq!(tower.crown_density, MODERATE_CANOPY_DENSITY);

	let PalmShadeItem::WaialeaPalm(lower) = PalmShadeCell::LowerWaialeaPalm.item() else {
		anyhow::bail!("expected lower waialea item");
	};
	assert_eq!(lower.height, UnitRange::new(8.0, 20.0));
	assert_eq!(lower.crown_density, DENSE_CANOPY_DENSITY);

	let PalmShadeItem::DatePalm(shade) = PalmShadeCell::ShadeDatePalm.item() else {
		anyhow::bail!("expected shade date palm item");
	};
	assert_eq!(shade.height, UnitRange::new(6.0, 20.0));
	assert_eq!(shade.crown_density, MODERATE_CANOPY_DENSITY);

	let PalmShadeItem::DatePalm(cluster) = PalmShadeCell::ClusterDatePalm.item() else {
		anyhow::bail!("expected cluster date palm item");
	};
	assert_eq!(cluster.height, UnitRange::new(6.0, 12.0));
	assert_eq!(cluster.crown_density, DENSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_match_rfc() -> Result<()> {
	let dist = PalmShadeCell::distribution();
	let tower = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(PalmShadeCell::TowerWaialeaPalm))
		.ok_or_else(|| anyhow::anyhow!("missing tower waialea bucket"))?;
	assert_eq!(tower.constraints.elevation.end, 0.46);
	assert_eq!(tower.constraints.steepness.end, 0.56);

	let lower = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(PalmShadeCell::LowerWaialeaPalm))
		.ok_or_else(|| anyhow::anyhow!("missing lower waialea bucket"))?;
	assert_eq!(lower.constraints.elevation.end, 0.50);
	assert_eq!(lower.constraints.steepness.end, 0.62);

	let shade = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(PalmShadeCell::ShadeDatePalm))
		.ok_or_else(|| anyhow::anyhow!("missing shade date palm bucket"))?;
	assert_eq!(shade.constraints.elevation.end, 0.52);
	assert_eq!(shade.constraints.steepness.end, 0.42);

	let cluster = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(PalmShadeCell::ClusterDatePalm))
		.ok_or_else(|| anyhow::anyhow!("missing cluster date palm bucket"))?;
	assert_eq!(cluster.constraints.elevation.end, 0.44);
	assert_eq!(cluster.constraints.steepness.end, 0.36);
	Ok(())
}

#[test]
fn steep_slope_rejects_cluster_date_but_allows_shade_date() -> Result<()> {
	let prepared =
		PalmShadeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.30, steepness: 0.38 };
	let shade_outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match shade_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, PalmShadeCell::ShadeDatePalm);
		}
		other => anyhow::bail!("expected ShadeDatePalm on moderate slope, got {other:?}"),
	}
	let cluster_outcome = prepared.select_from(
		5,
		Vec3::new(5.0, 0.30, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match cluster_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, PalmShadeCell::ClusterDatePalm);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		PalmShadeCell::TowerWaialeaPalm,
		PalmShadeCell::LowerWaialeaPalm,
		PalmShadeCell::ShadeDatePalm,
		PalmShadeCell::ClusterDatePalm,
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
