use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = ShamanhomeCell::distribution();
	assert_eq!(dist.len(), 8);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 6.0);
	assert_eq!(dist.buckets[1].item, Some(ShamanhomeCell::ShamanBraidOak));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(ShamanhomeCell::RedRitualBraidOak));
	assert_eq!(dist.buckets[2].weight, 0.45);
	assert_eq!(dist.buckets[3].item, Some(ShamanhomeCell::GnarledElderBraidOak));
	assert_eq!(dist.buckets[3].weight, 0.55);
	assert_eq!(dist.buckets[4].item, Some(ShamanhomeCell::SilverShrineBraidOak));
	assert_eq!(dist.buckets[4].weight, 0.30);
	assert_eq!(dist.buckets[5].item, Some(ShamanhomeCell::CopperBranchBraidOak));
	assert_eq!(dist.buckets[5].weight, 0.25);
	assert_eq!(dist.buckets[6].item, Some(ShamanhomeCell::RitualDatePalm));
	assert_eq!(dist.buckets[6].weight, 0.75);
	assert_eq!(dist.buckets[7].item, Some(ShamanhomeCell::SmallSopeBanyan));
	assert_eq!(dist.buckets[7].weight, 0.80);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = ShamanhomeCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.22..=0.48).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let ShamanhomeItem::BraidOak(oak) = ShamanhomeCell::ShamanBraidOak.item() else {
		anyhow::bail!("expected braid oak item");
	};
	assert_eq!(oak.height, UnitRange::new(4.0, 7.0));
	assert_eq!(oak.canopy_density, MODERATE_CANOPY_DENSITY);

	let ShamanhomeItem::BraidOak(elder) = ShamanhomeCell::GnarledElderBraidOak.item() else {
		anyhow::bail!("expected elder braid oak item");
	};
	assert_eq!(elder.height, UnitRange::new(5.0, 7.0));
	assert_eq!(elder.canopy_spread, UnitRange::new(2.0, 4.2));

	let ShamanhomeItem::BraidOak(shrine) = ShamanhomeCell::SilverShrineBraidOak.item() else {
		anyhow::bail!("expected shrine braid oak item");
	};
	assert_eq!(shrine.height, UnitRange::new(4.0, 6.0));
	assert_eq!(shrine.canopy_density, SPARSE_TO_MODERATE_CANOPY_DENSITY);

	let ShamanhomeItem::DatePalm(palm) = ShamanhomeCell::RitualDatePalm.item() else {
		anyhow::bail!("expected date palm item");
	};
	assert_eq!(palm.height, UnitRange::new(4.0, 6.0));

	let ShamanhomeItem::SopeBanyan(banyan) = ShamanhomeCell::SmallSopeBanyan.item() else {
		anyhow::bail!("expected sope banyan item");
	};
	assert_eq!(banyan.height, UnitRange::new(5.0, 7.0));
	assert_eq!(banyan.descender_density, SPARSE_DESCENDER_DENSITY);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn red_ritual_braid_oak_accepts_steeper_slope_than_ritual_date_palm() -> Result<()> {
	let prepared =
		ShamanhomeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.32 };
	let red_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match red_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, ShamanhomeCell::RedRitualBraidOak);
		}
		other => anyhow::bail!("expected RedRitualBraidOak on moderate slope, got {other:?}"),
	}
	let palm_outcome = prepared.select_from(
		6,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match palm_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, ShamanhomeCell::RitualDatePalm);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn high_elevation_rejects_date_palm_on_steep_slopes() -> Result<()> {
	let prepared =
		ShamanhomeCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.50, steepness: 0.15 };
	let outcome = prepared.select_from(
		6,
		Vec3::new(5.0, 0.50, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, ShamanhomeCell::RitualDatePalm);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
	let placements = grove.populate(&extent, &terrain);
	assert!(!placements.is_empty());

	let cell = definition().cell_extent_xz.x;
	let off_center = placements
		.iter()
		.filter(|p| {
			let local_x = (p.position.x / cell).fract() - 0.5;
			let local_z = (p.position.z / cell).fract() - 0.5;
			local_x.abs() > 0.25 || local_z.abs() > 0.25
		})
		.count();
	assert!(
		off_center * 2 >= placements.len(),
		"expected at least half of {} placements off cell centers, got {off_center}",
		placements.len()
	);
	Ok(())
}

#[test]
fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
	let grove = Grove::assemble(
		definition(),
		ForestGroveBiases::default(),
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(40.0, 1.0, 40.0));
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.12 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
