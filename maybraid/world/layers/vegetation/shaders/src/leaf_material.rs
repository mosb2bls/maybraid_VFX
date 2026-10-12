//! Canopy leaf [`Material`] — object-space leafy breakup, vertex sway, split light.
//!
//! A noisy rim `discard` runs near/mid only (Opaque ignores alpha).
//! Far cheap-ball hubs stay opaque so distant crowns do not punch to clear color.
//! Interior holes are near/mid only, in ball-radii (`abs / scale`, 80 radii)
//! so large cards keep the breakup as long as small ones of the same
//! apparent size.
//! Lambert + sky, plus fake canopy occlusion (inward faces / puff hubs).

use bevy::{
	asset::embedded_asset,
	light::NotShadowCaster,
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	reflect::TypePath,
	render::render_resource::{
		AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
	},
	shader::ShaderRef,
};

/// Registers embedded **`leaf_material.wgsl`** and [`MaterialPlugin`] for [`LeafMaterial`].
pub struct LeafMaterialPlugin;

impl Plugin for LeafMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "leaf_material.wgsl");
		app.add_plugins(MaterialPlugin::<LeafMaterial>::default());
		app.add_systems(PostUpdate, disable_leaf_shadow_casters);
	}
}

fn disable_leaf_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<LeafMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LeafMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl Default for LeafMaterial {
	fn default() -> Self {
		Self { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
	}
}

impl Material for LeafMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "leaf_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "leaf_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		// Opaque + `discard` holes. Alpha-to-coverage read as a window screen.
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		// ✅ Disable backface culling → renders both sides
		descriptor.primitive.cull_mode = None;

		Ok(())
	}
}

/// Cheese / sway LOD mirrored from `leaf_material.wgsl`.
///
/// `view_dist` is centroid meters divided by ball radius. Cheese stays on
/// while that ratio is below [`LEAF_MID_DIST`]. Keep this in lockstep with
/// the WGSL `LEAF_MID_DIST` / `view_dist` assignment.
#[cfg(test)]
mod cheese_lod {
	const LEAF_MID_DIST: f32 = 80.0;

	fn view_dist(abs_dist: f32, scale: f32) -> f32 {
		abs_dist / scale.max(1e-4)
	}

	fn cheese(abs_dist: f32, scale: f32) -> bool {
		view_dist(abs_dist, scale) < LEAF_MID_DIST
	}

	#[test]
	fn unit_kit_cheeses_to_mid_meters() {
		assert!(cheese(79.9, 1.0));
		assert!(!cheese(80.0, 1.0));
	}

	#[test]
	fn small_card_cut_scales_with_radius() {
		assert!(cheese(39.9, 0.5));
		assert!(!cheese(40.0, 0.5));
	}

	#[test]
	fn large_card_keeps_the_same_radii_window() {
		// Default Honu leaf ball is `tree_height * 0.10` = 2.4 m.
		// An absolute 140 m floor used to kill cheese here.
		assert!(cheese(140.0, 2.4));
		assert!(cheese(191.9, 2.4));
		assert!(!cheese(192.0, 2.4));
	}

	#[test]
	fn grove_proxy_does_not_go_rectangular_at_the_old_meter_cap() {
		assert!(cheese(140.0, 10.0));
		assert!(cheese(799.9, 10.0));
		assert!(!cheese(800.0, 10.0));
	}
}
