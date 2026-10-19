use super::*;
use crate::grove::{FlatTerrainSample, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = WanderingAcaciaCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(WanderingAcaciaCell::WanderingHighBush),
			Some(WanderingAcaciaCell::DryWanderingSopesBanyan),
			Some(WanderingAcaciaCell::WanderingVaseTree),
			Some(WanderingAcaciaCell::WanderingPenmarchTorch),
			Some(WanderingAcaciaCell::WanderingKamakuraTorch),
		]
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
	crate::grove::populate_checks::assert_populated_grove_is_deterministic_and_non_empty(
		definition(),
		crate::grove::GroveFrontend::default().noise,
		GroveExtent::new(Vec3::ZERO, Vec3::new(300.0, 1.0, 300.0)),
		&FlatTerrainSample::default(),
	)
}
