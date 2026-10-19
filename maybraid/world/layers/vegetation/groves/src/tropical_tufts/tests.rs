use super::*;
use crate::grove::{FlatTerrainSample, GroveCellOutcome, GroveExtent};
use anyhow::Result;
use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::NoiseParams;

#[test]
fn distribution_matches_rfc_order_and_membership() -> Result<()> {
	let dist = TropicalTuftsCell::distribution();
	assert_eq!(
		dist.buckets.iter().map(|b| b.item).collect::<Vec<_>>(),
		vec![
			None,
			Some(TropicalTuftsCell::BrightTuft),
			Some(TropicalTuftsCell::DeepTuft),
			Some(TropicalTuftsCell::YellowGreenTuft),
			Some(TropicalTuftsCell::SmallPalmBush),
			Some(TropicalTuftsCell::JuvenilePalmBush),
			Some(TropicalTuftsCell::BrightTuftPatch),
			Some(TropicalTuftsCell::DeepTuftPatch),
			Some(TropicalTuftsCell::YellowGreenTuftPatch),
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
fn patches_outweigh_single_tufts() -> Result<()> {
	let tuft_weight = |patch: bool| -> f32 {
		TropicalTuftsCell::distribution()
			.buckets
			.iter()
			.filter(|b| {
				b.item.is_some_and(|cell| match cell.item() {
					TropicalTuftsItem::Tuft(_) => !patch,
					TropicalTuftsItem::Patch(_) => patch,
					TropicalTuftsItem::PalmBush(_) => false,
				})
			})
			.map(|b| b.weight)
			.sum()
	};
	assert!(tuft_weight(true) > 2.0 * tuft_weight(false), "patches should dominate tuft weight");
	Ok(())
}

#[test]
fn variants_map_to_typed_items() -> Result<()> {
	assert!(matches!(TropicalTuftsCell::BrightTuft.item(), TropicalTuftsItem::Tuft(_)));
	let TropicalTuftsItem::PalmBush(palm) = TropicalTuftsCell::SmallPalmBush.item() else {
		anyhow::bail!("expected palm bush item");
	};
	assert_eq!(palm.frond_count, 4..=7);
	let TropicalTuftsItem::Patch(patch) = TropicalTuftsCell::BrightTuftPatch.item() else {
		anyhow::bail!("expected patch item");
	};
	assert_eq!(patch.clump, BRIGHT_TUFT);
	Ok(())
}

#[test]
fn first_fit_from_placed_bucket_places_variant() -> Result<()> {
	let prepared =
		TropicalTuftsCell::distribution().prepare(0.0, 0.0, NoiseParams::default(), Vec3::ZERO);
	let terrain = FlatTerrainSample { elevation: 0.4, steepness: 0.1 };
	let outcome = prepared.select_from(
		1,
		Vec3::new(5.0, 0.4, 5.0),
		1.0,
		Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
		&terrain,
	);
	assert!(matches!(outcome, GroveCellOutcome::Placed { .. }));
	Ok(())
}

#[test]
fn populated_grove_is_deterministic() -> Result<()> {
	crate::grove::populate_checks::assert_populated_grove_is_deterministic(
		definition(),
		NoiseParams::default(),
		GroveExtent::new(Vec3::ZERO, Vec3::new(20.0, 1.0, 20.0)),
		&FlatTerrainSample { elevation: 0.35, steepness: 0.15 },
	)?;
	Ok(())
}

#[cfg(feature = "render")]
mod render_tests {
	use super::*;
	use crate::grove::GroveCellVariant;
	use crate::tropical_tufts::TropicalTuftsParams;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::{Entity, Transform};
	use lod::SceneChunk;
	use lod::lod_ref::LodRef;
	use lod::scene::{LodScene, LodSceneLevel};
	use vegetation_components::VegetationComponents;

	#[test]
	fn palms_share_quantized_arcs() -> Result<()> {
		let mut params = TropicalTuftsParams::default();
		params.preview = params.preview.with_resolved_placements(vec![
			GroveCellVariant::new(TropicalTuftsCell::SmallPalmBush, Vec3::new(1.0, 0.0, 1.0), 1.0),
			GroveCellVariant::new(TropicalTuftsCell::SmallPalmBush, Vec3::new(4.0, 0.0, 1.0), 1.0),
		]);
		params.patch_variants = 1;
		let grove = params.build();
		assert_eq!(grove.palm_count(), 2);
		assert!(grove.plants().is_empty());
		assert!(grove.palms_share_unit_arc(), "same variant should share one PalmBush Arc");
		Ok(())
	}

	#[test]
	fn high_emits_lazy_grove_kits_including_posed_palms() -> Result<()> {
		use vegetation_components::FLATTENED_KIT_CHUNK_WEIGHT;

		let mut params = TropicalTuftsParams::default();
		params.preview = params.preview.with_resolved_placements(vec![
			GroveCellVariant::new(TropicalTuftsCell::BrightTuft, Vec3::new(1.0, 0.0, 2.0), 1.0),
			GroveCellVariant::new(TropicalTuftsCell::SmallPalmBush, Vec3::new(3.0, 0.0, 2.0), 1.0),
		]);
		let grove = params.build();
		assert_eq!(grove.plants().len(), 1);
		assert_eq!(grove.palm_count(), 1);
		let nodes = grove.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(nodes.len() > 1, "posed palm kits should join tuft foliage");

		let camera = Transform::from_translation(Vec3::new(1.0, 2.0, 8.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let high = grove.scene_chunks_with_level(&lod_ref, LodSceneLevel::High);
		assert!(
			!matches!(high, SceneChunk::SubChunks(ref parts) if parts.iter().any(|p| matches!(p, SceneChunk::SubChunks(_)))),
			"High should lazy-emit posed kits, not nested palm hosts"
		);
		assert_eq!(high.total_primitives(), nodes.len());
		assert_eq!(high.total_weight(), nodes.len() as u32 * FLATTENED_KIT_CHUNK_WEIGHT);
		Ok(())
	}
}
