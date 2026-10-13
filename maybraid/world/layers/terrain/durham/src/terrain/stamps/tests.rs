//! Unit tests for per-family jersey guillotine grids.

use crate::terrain::stamps::configs::TerrainStampConfigs;
use crate::terrain::stamps::plateau::{
	PlateauLowPassControllerCell, PlateauLowPassControllerLayout,
};
use crate::terrain::stamps::shared::{leaf_selected, LeafAabbs};
use crate::terrain::stamps::valley::{ValleyLowPassControllerCell, ValleyLowPassControllerLayout};
use anyhow::{bail, Result};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use lod::gen::Id;

fn assert_tiles_parent(parent: Aabb3d, leaves: &[Aabb3d]) -> Result<()> {
	if leaves.is_empty() {
		bail!("expected at least one leaf");
	}
	let parent_area = (parent.max.x - parent.min.x) * (parent.max.z - parent.min.z);
	let leaf_area: f32 = leaves.iter().map(|l| (l.max.x - l.min.x) * (l.max.z - l.min.z)).sum();
	if (leaf_area - parent_area).abs() > 1e-2 * parent_area.max(1.0) {
		bail!("area mismatch: parent={parent_area} leaves={leaf_area}");
	}
	Ok(())
}

#[test]
fn plateau_cuts_are_deterministic() -> Result<()> {
	let configs = TerrainStampConfigs::default();
	let cell = PlateauLowPassControllerLayout::default().cell_bounds(0, 0);
	let a = PlateauLowPassControllerCell::from_family_config(cell, &configs.plateau.low_pass);
	let b = PlateauLowPassControllerCell::from_family_config(cell, &configs.plateau.low_pass);
	assert_eq!(a.cuts, b.cuts);
	assert_eq!(a.leaf_aabbs(), b.leaf_aabbs());
	Ok(())
}

#[test]
fn plateau_leaves_tile_parent() -> Result<()> {
	let configs = TerrainStampConfigs::default();
	let cell = PlateauLowPassControllerLayout::default().cell_bounds(1, -2);
	let controller =
		PlateauLowPassControllerCell::from_family_config(cell, &configs.plateau.low_pass);
	let leaves = controller.leaf_aabbs();
	assert!(leaves.len() > 1, "expected multiple leaves, got {}", leaves.len());
	assert_tiles_parent(cell, &leaves)?;
	Ok(())
}

#[test]
fn family_controller_grids_are_offset_apart() -> Result<()> {
	let plateau = PlateauLowPassControllerLayout::default().cell_bounds(0, 0);
	let valley = ValleyLowPassControllerLayout::default().cell_bounds(0, 0);
	if (plateau.min.x - valley.min.x).abs() < 1e-3 && (plateau.min.z - valley.min.z).abs() < 1e-3 {
		bail!("expected plateau and valley controller origins to differ");
	}
	Ok(())
}

#[test]
fn valley_leaf_ids_stable() -> Result<()> {
	let configs = TerrainStampConfigs::default();
	let cell = ValleyLowPassControllerLayout::default().cell_bounds(0, 0);
	let controller =
		ValleyLowPassControllerCell::from_family_config(cell, &configs.valley.low_pass);
	let leaves = controller.leaf_aabbs();
	let ids: Vec<Id> = leaves.iter().copied().map(Id::from_cell).collect();
	let mut unique = ids.clone();
	unique.sort();
	unique.dedup();
	assert_eq!(unique.len(), ids.len(), "leaf Ids must be unique");
	let first = leaves.first().copied().ok_or_else(|| anyhow::anyhow!("no leaves"))?;
	let probe = Aabb3d::from_min_max(
		Vec3::new(
			(first.min.x + first.max.x) * 0.5 - 1.0,
			first.min.y,
			(first.min.z + first.max.z) * 0.5 - 1.0,
		),
		Vec3::new(
			(first.min.x + first.max.x) * 0.5 + 1.0,
			first.max.y,
			(first.min.z + first.max.z) * 0.5 + 1.0,
		),
	);
	assert!(leaves.iter().any(|leaf| probe.intersects(leaf)));
	Ok(())
}

#[test]
fn leaf_selected_respects_likelihood_extremes() -> Result<()> {
	let cell = Aabb3d::from_min_max(Vec3::new(0.0, -1.0, 0.0), Vec3::new(10.0, 1.0, 10.0));
	assert!(leaf_selected(cell, 123, 1.0, 100.0));
	assert!(!leaf_selected(cell, 123, 0.0, 100.0));
	Ok(())
}

#[test]
fn leaf_selected_is_spatially_correlated() -> Result<()> {
	let correlation = 2000.0;
	let seed = 99u32;
	let likelihood = 0.55;
	let mk = |x: f32, z: f32| {
		Aabb3d::from_min_max(Vec3::new(x, -1.0, z), Vec3::new(x + 100.0, 1.0, z + 100.0))
	};
	let a = leaf_selected(mk(0.0, 0.0), seed, likelihood, correlation);
	let near = leaf_selected(mk(80.0, 0.0), seed, likelihood, correlation);
	assert_eq!(a, near, "nearby leaves should share occupancy for long correlation");
	Ok(())
}

#[test]
fn layout_defaults_feed_configs() -> Result<()> {
	use crate::terrain::stamps::configs::TerrainStampConfigs;
	use crate::terrain::stamps::massif::MassifLowPassControllerLayout;
	let configs = TerrainStampConfigs::default();
	assert_eq!(configs.massif.low_pass.likelihood, MassifLowPassControllerLayout::LIKELIHOOD);
	assert_eq!(
		configs.massif.low_pass.spatial_correlation,
		MassifLowPassControllerLayout::SPATIAL_CORRELATION
	);
	assert_eq!(
		configs.massif.low_pass.guillotine.step_min,
		MassifLowPassControllerLayout::CELL_SIZE_MIN
	);
	assert_eq!(
		configs.massif.low_pass.guillotine.step_max,
		MassifLowPassControllerLayout::CELL_SIZE_MAX
	);
	assert_eq!(configs.massif.low_pass.strength_min, MassifLowPassControllerLayout::STRENGTH_MIN);
	assert_eq!(configs.massif.low_pass.strength_max, MassifLowPassControllerLayout::STRENGTH_MAX);
	Ok(())
}

#[test]
fn sample_strength_stays_in_range() -> Result<()> {
	use crate::terrain::stamps::shared::sample_strength;
	for seed in 0..64u32 {
		let s = sample_strength(seed, 0.5, 1.5);
		assert!((0.5..=1.5).contains(&s), "seed={seed} s={s}");
	}
	assert_eq!(sample_strength(1, 1.0, 1.0), 1.0);
	Ok(())
}

#[test]
fn high_pass_cells_are_much_larger_than_low_pass() -> Result<()> {
	use crate::terrain::stamps::massif::{
		MassifHighPassControllerLayout, MassifLowPassControllerLayout,
	};
	assert!(
		MassifHighPassControllerLayout::CELL_SIZE_MAX
			> MassifLowPassControllerLayout::CELL_SIZE_MAX * 4.0
	);
	let low = MassifLowPassControllerLayout::default().grid.cell_size;
	let high = MassifHighPassControllerLayout::default().grid.cell_size;
	assert!(high > low * 5.0, "high={high} low={low}");
	Ok(())
}

#[test]
fn from_world_seed_42_matches_default_family_seeds() {
	let configs = TerrainStampConfigs::from_world_seed(42);
	assert_eq!(configs.plateau.low_pass.seed, 42);
	assert_eq!(configs.plateau.high_pass.seed, 1042);
	assert_eq!(configs.valley.low_pass.seed, 47);
	assert_eq!(configs.valley.high_pass.seed, 1047);
}
