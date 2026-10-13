use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = StrangeOasisCell::distribution();
	assert_eq!(dist.len(), 5);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 10.0);
	assert_eq!(dist.buckets[1].item, Some(StrangeOasisCell::CompactDatePalm));
	assert_eq!(dist.buckets[1].weight, 2.0);
	assert_eq!(dist.buckets[2].item, Some(StrangeOasisCell::TorchAccent));
	assert_eq!(dist.buckets[2].weight, 0.30);
	assert_eq!(dist.buckets[3].item, Some(StrangeOasisCell::RedTorchAccent));
	assert_eq!(dist.buckets[3].weight, 0.18);
	assert_eq!(dist.buckets[4].item, Some(StrangeOasisCell::OasisStorybook));
	assert_eq!(dist.buckets[4].weight, 0.75);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = StrangeOasisCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.08..=0.25).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let StrangeOasisItem::DatePalm(palm) = StrangeOasisCell::CompactDatePalm.item() else {
		anyhow::bail!("expected date palm item");
	};
	assert_eq!(palm.height, UnitRange::new(3.0, 5.0));
	assert_eq!(palm.crown_density, MODERATE_CANOPY_DENSITY);

	let StrangeOasisItem::Storybook(story) = StrangeOasisCell::OasisStorybook.item() else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(4.0, 6.0));

	let StrangeOasisItem::Torch(torch) = StrangeOasisCell::RedTorchAccent.item() else {
		anyhow::bail!("expected red torch item");
	};
	assert_eq!(torch.height, UnitRange::new(3.0, 6.5));
	assert_eq!(torch.canopy_density, SPARSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn red_torch_accepts_steeper_slope_than_compact_date_palm() -> Result<()> {
	let prepared =
		StrangeOasisCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.32 };
	let red_outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match red_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, StrangeOasisCell::RedTorchAccent);
		}
		other => anyhow::bail!("expected RedTorchAccent on moderate slope, got {other:?}"),
	}
	let palm_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.25, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match palm_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, StrangeOasisCell::CompactDatePalm);
		}
		GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => {}
	}
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn high_elevation_rejects_oasis_floor_variants() -> Result<()> {
	let prepared =
		StrangeOasisCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.45, steepness: 0.15 };
	let outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.45, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_ne!(variant, StrangeOasisCell::CompactDatePalm);
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
