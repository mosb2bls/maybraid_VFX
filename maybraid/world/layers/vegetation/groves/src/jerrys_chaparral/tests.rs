use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = JerrysChaparralCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(JerrysChaparralCell::DryRoryHeadTrained),
			Some(JerrysChaparralCell::ChaparralHighBush),
			Some(JerrysChaparralCell::SmallFriendsConifer),
			Some(JerrysChaparralCell::ManzanitaRory),
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
	let dist = JerrysChaparralCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.24..=0.52).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn rory_bush_and_conifer_placed_weights_match_rfc_ratio() -> Result<()> {
	let weight = |kind: &str| -> f32 {
		JerrysChaparralCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match (kind, cell.item()) {
					("rory", JerrysChaparralItem::RoryHead(_)) => true,
					("bush", JerrysChaparralItem::Bush(_)) => true,
					("conifer", JerrysChaparralItem::FriendsConifer(_)) => true,
					_ => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	let rory = weight("rory");
	let bush = weight("bush");
	let conifer = weight("conifer");
	assert!((rory - 1.85).abs() < 1e-4, "expected rory weight 1.85, got {rory}");
	assert!((bush - 2.0).abs() < 1e-4, "expected bush weight 2.0, got {bush}");
	assert!((conifer - 0.45).abs() < 1e-4, "expected conifer weight 0.45, got {conifer}");
	Ok(())
}

#[test]
fn rory_bush_and_conifer_geometry_follows_authored_bands() -> Result<()> {
	let JerrysChaparralItem::RoryHead(dry) = JerrysChaparralCell::DryRoryHeadTrained.item() else {
		anyhow::bail!("expected dry rory item");
	};
	assert!(dry.height.start >= 1.20);
	assert!(dry.height.end <= 3.20);

	let JerrysChaparralItem::Bush(bush) = JerrysChaparralCell::ChaparralHighBush.item() else {
		anyhow::bail!("expected bush item");
	};
	assert_eq!(bush.shoot_count, 7..=11);
	assert!(bush.leaf_radius.end <= 0.11);

	let JerrysChaparralItem::FriendsConifer(conifer) =
		JerrysChaparralCell::SmallFriendsConifer.item()
	else {
		anyhow::bail!("expected conifer item");
	};
	assert!(conifer.height.end <= 6.00);

	let JerrysChaparralItem::RoryHead(manzanita) = JerrysChaparralCell::ManzanitaRory.item() else {
		anyhow::bail!("expected manzanita rory item");
	};
	assert!(manzanita.canopy_density.end <= 0.35);
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
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.15 };
	let a = grove.populate(&extent, &terrain);
	let b = grove.populate(&extent, &terrain);
	assert_eq!(a, b);
	assert!(!a.is_empty());
	Ok(())
}
