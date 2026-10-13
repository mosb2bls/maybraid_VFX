use super::*;
use crate::grove::{FlatTerrainSample, ForestGroveBiases, Grove, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_weights() -> Result<()> {
	let dist = TropicalUndergrowthCell::distribution();
	assert_eq!(dist.len(), 12);
	assert!(dist.buckets[0].item.is_none());
	assert_eq!(dist.buckets[0].weight, 12.0);
	assert_eq!(dist.buckets[1].item, Some(TropicalUndergrowthCell::BrightTuft));
	assert_eq!(dist.buckets[1].weight, 0.4);
	assert_eq!(dist.buckets[2].item, Some(TropicalUndergrowthCell::DeepTuft));
	assert_eq!(dist.buckets[2].weight, 0.3);
	assert_eq!(dist.buckets[3].item, Some(TropicalUndergrowthCell::SmallPalmBush));
	assert_eq!(dist.buckets[3].weight, 1.0);
	assert_eq!(dist.buckets[4].item, Some(TropicalUndergrowthCell::MiniRoryHeadTrained));
	assert_eq!(dist.buckets[4].weight, 0.85);
	assert_eq!(dist.buckets[5].item, Some(TropicalUndergrowthCell::MiniVaseTree));
	assert_eq!(dist.buckets[5].weight, 0.20);
	assert_eq!(dist.buckets[6].item, Some(TropicalUndergrowthCell::MiniSparseStorybook));
	assert_eq!(dist.buckets[6].weight, 0.15);
	assert_eq!(dist.buckets[7].item, Some(TropicalUndergrowthCell::MiniPenmarchTorch));
	assert_eq!(dist.buckets[7].weight, 1.30);
	assert_eq!(dist.buckets[8].item, Some(TropicalUndergrowthCell::MiniKamakuraTorch));
	assert_eq!(dist.buckets[8].weight, 1.22);
	assert_eq!(dist.buckets[9].item, Some(TropicalUndergrowthCell::MiniTorchTree));
	assert_eq!(dist.buckets[9].weight, 0.22);
	assert_eq!(dist.buckets[10].item, Some(TropicalUndergrowthCell::BrightTuftPatch));
	assert_eq!(dist.buckets[10].weight, 1.6);
	assert_eq!(dist.buckets[11].item, Some(TropicalUndergrowthCell::DeepTuftPatch));
	assert_eq!(dist.buckets[11].weight, 1.2);
	Ok(())
}

#[test]
fn placed_share_sits_in_rfc_density_range() -> Result<()> {
	let dist = TropicalUndergrowthCell::distribution();
	let total: f32 = dist.buckets.iter().map(|b| b.weight).sum();
	let placed: f32 = dist.buckets.iter().filter(|b| b.item.is_some()).map(|b| b.weight).sum();
	let share = placed / total;
	assert!((0.22..=0.58).contains(&share), "placed share {share} outside RFC density");
	Ok(())
}

#[test]
fn patches_outweigh_single_tufts() -> Result<()> {
	let tuft_weight = |patch: bool| -> f32 {
		TropicalUndergrowthCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match cell.item() {
					TropicalUndergrowthItem::Tuft(_) => !patch,
					TropicalUndergrowthItem::Patch(_) => patch,
					_ => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(tuft_weight(true) > 2.0 * tuft_weight(false), "patches should dominate tuft weight");
	Ok(())
}

#[test]
fn tuft_palm_and_tree_placed_weights_match_rfc_ratio() -> Result<()> {
	let weight = |kind: &str| -> f32 {
		TropicalUndergrowthCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match (kind, cell.item()) {
					(
						"tuft",
						TropicalUndergrowthItem::Tuft(_) | TropicalUndergrowthItem::Patch(_),
					) => true,
					("palm", TropicalUndergrowthItem::PalmBush(_)) => true,
					("rory", TropicalUndergrowthItem::RoryHead(_)) => true,
					("vase", TropicalUndergrowthItem::VaseTree(_)) => true,
					("story", TropicalUndergrowthItem::Storybook(_)) => true,
					(
						"torch",
						TropicalUndergrowthItem::PenmarchTorch(_)
						| TropicalUndergrowthItem::KamakuraTorch(_)
						| TropicalUndergrowthItem::TorchTree(_),
					) => true,
					_ => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	let tuft = weight("tuft");
	let palm = weight("palm");
	let rory = weight("rory");
	let vase = weight("vase");
	let story = weight("story");
	let torch = weight("torch");
	assert!((tuft - 3.5).abs() < 1e-4, "expected tuft weight 3.5, got {tuft}");
	assert!((palm - 1.0).abs() < 1e-4, "expected palm weight 1.0, got {palm}");
	assert!((rory - 0.85).abs() < 1e-4, "expected rory weight 0.85, got {rory}");
	assert!((vase - 0.20).abs() < 1e-4, "expected vase weight 0.20, got {vase}");
	assert!((story - 0.15).abs() < 1e-4, "expected story weight 0.15, got {story}");
	assert!((torch - 2.74).abs() < 1e-4, "expected torch weight 2.74, got {torch}");
	Ok(())
}

#[test]
fn tuft_geometry_follows_authored_bands() -> Result<()> {
	let TropicalUndergrowthItem::Tuft(bright) = TropicalUndergrowthCell::BrightTuft.item() else {
		anyhow::bail!("expected bright tuft item");
	};
	assert!(bright.height.start >= 0.30);
	assert!(bright.height.end <= 1.50);

	let TropicalUndergrowthItem::Tuft(deep) = TropicalUndergrowthCell::DeepTuft.item() else {
		anyhow::bail!("expected deep tuft item");
	};
	assert!(deep.height.start >= 0.40);
	assert!(deep.height.end <= 0.90);
	Ok(())
}

#[test]
fn palm_and_mini_tree_geometry_follows_authored_bands() -> Result<()> {
	let TropicalUndergrowthItem::PalmBush(palm) = TropicalUndergrowthCell::SmallPalmBush.item()
	else {
		anyhow::bail!("expected palm item");
	};
	assert!(palm.height.start >= 1.00);
	assert!(palm.height.end <= 2.80);
	assert_eq!(palm.frond_count, 5..=9);

	let TropicalUndergrowthItem::RoryHead(rory) =
		TropicalUndergrowthCell::MiniRoryHeadTrained.item()
	else {
		anyhow::bail!("expected rory item");
	};
	assert!(rory.height.start >= 0.80);
	assert!(rory.height.end <= 1.80);
	assert!(rory.stalk_radius.start >= 0.037);
	assert!(rory.stalk_radius.end <= 0.055);
	assert!(rory.canopy_spread.start >= 0.70);
	assert!(rory.canopy_density.end <= 1.0);

	let TropicalUndergrowthItem::VaseTree(vase) = TropicalUndergrowthCell::MiniVaseTree.item()
	else {
		anyhow::bail!("expected vase item");
	};
	assert!(vase.height.start >= 1.00);
	assert!(vase.height.end <= 2.30);
	assert!(vase.stalk_radius.start >= 0.046);
	assert!(vase.canopy_spread.end <= 2.10);

	let TropicalUndergrowthItem::Storybook(story) =
		TropicalUndergrowthCell::MiniSparseStorybook.item()
	else {
		anyhow::bail!("expected storybook item");
	};
	assert!(story.height.start >= 1.20);
	assert!(story.height.end <= 2.50);
	assert!(story.stalk_radius.end <= 0.063);
	assert!(story.canopy_spread.start >= 0.84);
	Ok(())
}

#[test]
fn patch_wraps_bright_tuft_clump() -> Result<()> {
	let TropicalUndergrowthItem::Patch(patch) = TropicalUndergrowthCell::BrightTuftPatch.item()
	else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, BRIGHT_TUFT);
	assert!(*patch.clump_count.start() >= 3);
	assert!(patch.patch_extent_xz.start >= 1.0);
	Ok(())
}

#[test]
#[ignore = "placement constraints deferred to forest-layer normalization"]
fn constraint_first_fit_fallback() -> Result<()> {
	// SmallPalmBush (index 3) rejects steepness 0.65; first-fit falls to MiniRoryHeadTrained
	// (index 4), which allows steepness up to 0.70.
	let prepared = TropicalUndergrowthCell::distribution().prepare(
		0.0,
		0.0,
		NoiseParams::default(),
		Vec3::ZERO,
	);
	let terrain = FlatTerrainSample { elevation: 0.35, steepness: 0.65 };
	let outcome = prepared.select_from(
		3,
		Vec3::new(5.0, 0.35, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	match outcome {
		GroveCellOutcome::Placed { variant, .. } => {
			assert_eq!(variant, TropicalUndergrowthCell::MiniRoryHeadTrained);
		}
		other => anyhow::bail!("expected MiniRoryHeadTrained fallback, got {other:?}"),
	}
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
