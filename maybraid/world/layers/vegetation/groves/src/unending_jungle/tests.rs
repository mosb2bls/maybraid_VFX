use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = UnendingJungleCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(UnendingJungleCell::SmallHonuBanyan),
			Some(UnendingJungleCell::SmallSopeBanyan),
			Some(UnendingJungleCell::LowerStorybook),
			Some(UnendingJungleCell::SmallJungleStorybook),
			Some(UnendingJungleCell::PenmarchAccent),
			Some(UnendingJungleCell::RedJungleTorch),
			Some(UnendingJungleCell::RoryAccent),
			Some(UnendingJungleCell::WaialeaPalmAccent),
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
	let dist = UnendingJungleCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.24..=0.52).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn geometry_follows_authored_bands() -> Result<()> {
	let UnendingJungleItem::Honu(honu) = UnendingJungleCell::SmallHonuBanyan.item() else {
		anyhow::bail!("expected honu item");
	};
	assert_eq!(honu.height, UnitRange::new(4.0, 6.0));
	assert_eq!(honu.canopy_density, MODERATE_CANOPY_DENSITY);

	let UnendingJungleItem::Storybook(story) = UnendingJungleCell::LowerStorybook.item() else {
		anyhow::bail!("expected storybook item");
	};
	assert_eq!(story.height, UnitRange::new(3.0, 5.0));

	let UnendingJungleItem::JungleStorybook(jungle) =
		UnendingJungleCell::SmallJungleStorybook.item()
	else {
		anyhow::bail!("expected jungle storybook item");
	};
	assert_eq!(jungle.height, UnitRange::new(6.0, 8.0));
	assert_eq!(jungle.canopy_density, DENSE_CANOPY_DENSITY);

	let UnendingJungleItem::WaialeaPalm(palm) = UnendingJungleCell::WaialeaPalmAccent.item() else {
		anyhow::bail!("expected waialea item");
	};
	assert_eq!(palm.height, UnitRange::new(6.0, 9.0));
	Ok(())
}

#[test]
fn rory_accepts_steeper_slope_than_dense_storybook() -> Result<()> {
	let dist = UnendingJungleCell::distribution();
	let rory = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(UnendingJungleCell::RoryAccent))
		.ok_or_else(|| anyhow::anyhow!("missing rory bucket"))?;
	let jungle = dist
		.buckets
		.iter()
		.find(|b| b.item == Some(UnendingJungleCell::SmallJungleStorybook))
		.ok_or_else(|| anyhow::anyhow!("missing jungle storybook bucket"))?;
	assert!(rory.constraints.steepness.end > jungle.constraints.steepness.end);
	assert_eq!(jungle.constraints.steepness.end, 0.58);
	assert_eq!(rory.constraints.steepness.end, 0.76);
	Ok(())
}

#[test]
fn placements_break_the_cell_grid() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let placements = grove.populate(&extent, &terrain);
	assert!(!placements.is_empty());

	let cell = definition().cell_extent_xz.x;
	let off_center = placements.iter().any(|placed| {
		let offset_x = (placed.position.x % cell).abs();
		let offset_z = (placed.position.z % cell).abs();
		!(offset_x < 0.5 || (cell - offset_x) < 0.5) || !(offset_z < 0.5 || (cell - offset_z) < 0.5)
	});
	assert!(off_center, "expected placements offset from cell centers");
	Ok(())
}

#[test]
fn populated_grove_is_deterministic_and_non_empty() -> Result<()> {
	let noise = crate::grove::GroveFrontend::default().noise;
	let grove = Grove::assemble(definition(), ForestGroveBiases::default(), noise, Vec3::ZERO);
	let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(50.0, 1.0, 50.0));
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
