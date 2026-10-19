use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = DrylandCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![None, Some(DrylandCell::DrylandLiamsConifer), Some(DrylandCell::DrylandVaseTree),]
	);
	for bucket in &dist.buckets {
		assert!(
			bucket.weight.is_finite() && bucket.weight >= 0.0,
			"weight {} must be finite and nonnegative",
			bucket.weight
		);
	}
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = DrylandCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.03..=0.12).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let DrylandItem::LiamsConifer(liams) = DrylandCell::DrylandLiamsConifer.item() else {
		anyhow::bail!("expected liams item");
	};
	assert_eq!(liams.height, UnitRange::new(10.0, 20.0));
	assert_eq!(liams.canopy_density, SPARSE_CANOPY_DENSITY);

	let DrylandItem::VaseTree(vase) = DrylandCell::DrylandVaseTree.item() else {
		anyhow::bail!("expected vase item");
	};
	assert_eq!(vase.height, UnitRange::new(10.0, 20.0));
	assert_eq!(vase.canopy_density, SPARSE_CANOPY_DENSITY);
	Ok(())
}

#[test]
fn placement_constraints_use_full_elevation_and_rfc_steepness() -> Result<()> {
	let dist = DrylandCell::distribution();
	for bucket in dist.buckets.iter().filter(|b| b.item.is_some()) {
		assert_eq!(bucket.constraints.elevation.start, 0.0);
		assert_eq!(bucket.constraints.elevation.end, 1.0);
	}
	let liams = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(DrylandCell::DrylandLiamsConifer))
		.ok_or_else(|| anyhow::anyhow!("missing liams bucket"))?;
	assert_eq!(liams.constraints.steepness.end, 0.82);

	let vase = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(DrylandCell::DrylandVaseTree))
		.ok_or_else(|| anyhow::anyhow!("missing vase bucket"))?;
	assert_eq!(vase.constraints.steepness.end, 0.70);
	Ok(())
}

#[test]
fn palette_resolves_for_all_varietals() -> Result<()> {
	for cell in [DrylandCell::DrylandLiamsConifer, DrylandCell::DrylandVaseTree] {
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
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(280.0, 1.0, 280.0));
	let terrain = FlatTerrainSample::default();
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
