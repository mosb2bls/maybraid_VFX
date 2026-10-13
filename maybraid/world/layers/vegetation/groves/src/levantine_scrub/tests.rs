use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = LevantineScrubCell::distribution();
	assert_eq!(dist.len(), 8);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 11.0);
	assert_eq!(dist.buckets[1].item, Some(LevantineScrubCell::DryRoryHeadTrained));
	assert_eq!(dist.buckets[1].weight, 1.2);
	assert_eq!(dist.buckets[2].item, Some(LevantineScrubCell::SmallVaseTree));
	assert_eq!(dist.buckets[2].weight, 0.70);
	assert_eq!(dist.buckets[3].item, Some(LevantineScrubCell::DryHighBush));
	assert_eq!(dist.buckets[3].weight, 2.0);
	assert_eq!(dist.buckets[4].item, Some(LevantineScrubCell::SmallPenmarchTorch));
	assert_eq!(dist.buckets[4].weight, 0.45);
	assert_eq!(dist.buckets[5].item, Some(LevantineScrubCell::RedOliveTorch));
	assert_eq!(dist.buckets[5].weight, 0.25);
	assert_eq!(dist.buckets[6].item, Some(LevantineScrubCell::SmallBraidOak));
	assert_eq!(dist.buckets[6].weight, 0.35);
	assert_eq!(dist.buckets[7].item, Some(LevantineScrubCell::ScrubHedge));
	assert_eq!(dist.buckets[7].weight, 0.50);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = LevantineScrubCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.18..=0.48).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn scrub_geometry_follows_authored_bands() -> Result<()> {
	let LevantineScrubItem::RoryHead(rory) = LevantineScrubCell::DryRoryHeadTrained.item() else {
		anyhow::bail!("expected dry rory item");
	};
	assert!(rory.canopy_density.end <= 0.35);

	let LevantineScrubItem::VaseTree(vase) = LevantineScrubCell::SmallVaseTree.item() else {
		anyhow::bail!("expected vase item");
	};
	assert!(vase.height.end <= 3.00);

	let LevantineScrubItem::Bush(bush) = LevantineScrubCell::DryHighBush.item() else {
		anyhow::bail!("expected bush item");
	};
	assert_eq!(bush.shoot_count, 7..=11);

	let LevantineScrubItem::Hedge(hedge) = LevantineScrubCell::ScrubHedge.item() else {
		anyhow::bail!("expected hedge item");
	};
	assert!(hedge.width.end <= 1.80);
	Ok(())
}

#[test]
fn hedge_accepts_gentle_slopes_only() -> Result<()> {
	let prepared =
		LevantineScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.30 };
	let outcome = prepared.select_from(
		7,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, LevantineScrubCell::ScrubHedge);
		}
		other => anyhow::bail!("expected ScrubHedge on gentle slope, got {other:?}"),
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_all_placed_buckets() -> Result<()> {
	let prepared =
		LevantineScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.69 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Empty { .. } => {}
		other => anyhow::bail!("expected Empty on steep slope, got {other:?}"),
	}
	Ok(())
}

#[test]
fn bush_fits_moderate_slope_from_high_bush_bucket() -> Result<()> {
	let prepared =
		LevantineScrubCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.62 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, LevantineScrubCell::DryHighBush);
		}
		other => anyhow::bail!("expected DryHighBush, got {other:?}"),
	}
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
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
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
