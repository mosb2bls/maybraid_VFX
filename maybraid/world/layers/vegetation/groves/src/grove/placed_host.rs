//! Unused nested-host path: [`vegetation_components::ComponentsOnly`]`<`[`PlacedVegetation`]`<T>>`.
//!
//! Live groves pose plants with [`super::vc_compose::nest_flattened_plant_host`]. These
//! helpers are extracted so they are not the documented compose surface.
#![allow(dead_code)]

use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::MaterialRef;
use vegetation_components::{
	components_only_host, PlacedVegetation, Placement, VegetationComponents,
};

/// Nest one posed plant as [`vegetation_components::ComponentsOnly`]`<`[`PlacedVegetation`]`<T>>`.
pub fn nest_placed_plant_host<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> impl Scene + 'static
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	components_only_host(
		PlacedVegetation::new(
			plant,
			placement,
			stick_material.clone(),
			ball_material.clone(),
			frond_material.clone(),
		),
		lod_ref,
	)
}

/// Weighted chunk wrapping [`nest_placed_plant_host`].
pub fn nest_placed_plant_chunk<T>(
	plant: T,
	placement: Placement,
	stick_material: &MaterialRef,
	ball_material: &MaterialRef,
	frond_material: &MaterialRef,
	lod_ref: &LodRef,
) -> SceneChunk
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	SceneChunk::weighted(
		1,
		nest_placed_plant_host(
			plant,
			placement,
			stick_material,
			ball_material,
			frond_material,
			lod_ref,
		),
	)
}
