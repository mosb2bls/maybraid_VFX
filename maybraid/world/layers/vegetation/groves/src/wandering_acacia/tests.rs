use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = WanderingAcaciaCell::distribution();
	assert_eq!(dist.len(), 6);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 37.0);
	assert_eq!(dist.buckets[1].item, Some(WanderingAcaciaCell::WanderingHighBush));
	assert_eq!(dist.buckets[1].weight, 5.0);
	assert_eq!(dist.buckets[2].item, Some(WanderingAcaciaCell::DryWanderingSopesBanyan));
	assert_eq!(dist.buckets[2].weight, 1.0);
	assert_eq!(dist.buckets[3].item, Some(WanderingAcaciaCell::WanderingVaseTree));
	assert_eq!(dist.buckets[3].weight, 0.25);
	assert_eq!(dist.buckets[4].item, Some(WanderingAcaciaCell::WanderingPenmarchTorch));
	assert_eq!(dist.buckets[4].weight, 0.18);
	assert_eq!(dist.buckets[5].item, Some(WanderingAcaciaCell::WanderingKamakuraTorch));
	assert_eq!(dist.buckets[5].weight, 0.12);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = WanderingAcaciaCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.08..=0.24).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let WanderingAcaciaItem::HighBush(bush) = WanderingAcaciaCell::WanderingHighBush.item() else {
		anyhow::bail!("expected wandering high bush item");
	};
	assert_eq!(bush.height, UnitRange::new(5.0, 15.0));
	assert_eq!(bush.leaf_radius, UnitRange::new(0.45, 0.72));
	assert_eq!(bush.radial_strength, SPARSE_PROJECTION_RADIAL);

	let WanderingAcaciaItem::Sope(sope) = WanderingAcaciaCell::DryWanderingSopesBanyan.item()
	else {
		anyhow::bail!("expected dry wandering sope item");
	};
	assert_eq!(sope.height, UnitRange::new(5.0, 20.0));
	assert_eq!(sope.descender_density, SPARSE_DESCENDER_DENSITY);

	let WanderingAcaciaItem::VaseTree(vase) = WanderingAcaciaCell::WanderingVaseTree.item() else {
		anyhow::bail!("expected wandering vase item");
	};
	assert_eq!(vase.height, UnitRange::new(4.0, 8.0));
	assert_eq!(vase.canopy_density, SPARSE_CANOPY_DENSITY);

	let WanderingAcaciaItem::PenmarchTorch(torch) =
		WanderingAcaciaCell::WanderingPenmarchTorch.item()
	else {
		anyhow::bail!("expected wandering penmarch item");
	};
	assert_eq!(torch.height, UnitRange::new(5.0, 8.0));
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = WanderingAcaciaCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let bush = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(WanderingAcaciaCell::WanderingHighBush))
		.ok_or_else(|| anyhow::anyhow!("missing wandering high bush bucket"))?;
	assert_eq!(bush.constraints.steepness.end, 0.66);

	let sope = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(WanderingAcaciaCell::DryWanderingSopesBanyan))
		.ok_or_else(|| anyhow::anyhow!("missing dry wandering sope bucket"))?;
	assert_eq!(sope.constraints.steepness.end, 0.58);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn steep_slope_rejects_dry_sope_but_falls_through_to_vase() -> Result<()> {
	let prepared =
		WanderingAcaciaCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let moderate = FlatTerrainSample { elevation: 0.40, steepness: 0.55 };
	let sope_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&moderate,
	);
	match sope_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, WanderingAcaciaCell::DryWanderingSopesBanyan);
		}
		other => {
			anyhow::bail!("expected DryWanderingSopesBanyan on moderate slope, got {other:?}")
		}
	}
	let steep = FlatTerrainSample { elevation: 0.40, steepness: 0.60 };
	let steep_outcome = prepared.select_from(
		2,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match steep_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, WanderingAcaciaCell::WanderingVaseTree);
		}
		other => anyhow::bail!(
			"expected fall-through to WanderingVaseTree on steep slope, got {other:?}"
		),
	}
	let bush_outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.40, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&steep,
	);
	match bush_outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, WanderingAcaciaCell::WanderingHighBush);
		}
		other => anyhow::bail!("expected WanderingHighBush on steep slope, got {other:?}"),
	}
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [
		WanderingAcaciaCell::WanderingHighBush,
		WanderingAcaciaCell::DryWanderingSopesBanyan,
		WanderingAcaciaCell::WanderingVaseTree,
		WanderingAcaciaCell::WanderingPenmarchTorch,
		WanderingAcaciaCell::WanderingKamakuraTorch,
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
