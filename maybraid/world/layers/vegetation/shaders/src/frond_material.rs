//! Frond [`Material`] — palette color, tip-weighted vertex sway, double-sided PBR.
//!
//! Fragment stays opaque: no FBM cheese and no `discard`. Authored kit silhouettes
//! (straight-frond GLBs) are the edge. Sway reads kit-local from vertex COLOR after
//! collection merge (the same pack cheap-ball collections use).

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

/// Registers embedded **`frond_material.wgsl`** and [`MaterialPlugin`] for [`FrondMaterial`].
pub struct FrondMaterialPlugin;

impl Plugin for FrondMaterialPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "frond_material.wgsl");
		app.add_plugins(MaterialPlugin::<FrondMaterial>::default());
		app.add_systems(PostUpdate, disable_frond_shadow_casters);
	}
}

fn disable_frond_shadow_casters(
	mut commands: Commands,
	query: Query<Entity, (With<MeshMaterial3d<FrondMaterial>>, Without<NotShadowCaster>)>,
) {
	for entity in &query {
		commands.entity(entity).insert(NotShadowCaster);
	}
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FrondMaterial {
	#[uniform(0)]
	pub base_color: Vec4,
}

impl Default for FrondMaterial {
	fn default() -> Self {
		Self { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
	}
}

impl Material for FrondMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "frond_material.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "frond_material.wgsl").into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		// Custom interpolators (`local_pos`, `view_dist`) do not match the default
		// prepass vertex layout. Opaque + no `discard` still keeps early-Z in forward.
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = None;
		Ok(())
	}
}
