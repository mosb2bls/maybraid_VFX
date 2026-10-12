///! Presentation materials for visual style upscale treatments.

use bevy::asset::embedded_asset;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
	AsBindGroup, Face, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use super::visual_style::PresentationTreatment;

/// Parameters passed to presentation shaders.
#[derive(Clone, Copy, Debug, ShaderType)]
struct TreatmentParams {
	source_size: Vec2,
	target_size: Vec2,
}

/// Impressionist presentation material: linear upscale with painterly softening.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct ImpressionistMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material for ImpressionistMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "presentation.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#impressionist_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

impl ImpressionistMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Pre-post-modern presentation material: misregistered color-print effect.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PrePostModernMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material for PrePostModernMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "presentation.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#pre_post_modern_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

impl PrePostModernMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Nearest-neighbor presentation material: single nearest sample, no filtering.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct NearestNeighborMaterial {
	#[texture(0)]
	#[sampler(1)]
	pub source: Handle<Image>,
	#[uniform(2)]
	params: TreatmentParams,
}

impl Material for NearestNeighborMaterial {
	fn vertex_shader() -> ShaderRef {
		concat!("embedded://", env!("CARGO_CRATE_NAME"), "/", "presentation.wgsl").into()
	}

	fn fragment_shader() -> ShaderRef {
		concat!(
			"embedded://",
			env!("CARGO_CRATE_NAME"),
			"/",
			"presentation.wgsl#nearest_neighbor_fragment"
		)
		.into()
	}

	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Opaque
	}

	fn reads_view_transmission_texture(&self) -> bool {
		false
	}

	fn enable_prepass() -> bool {
		false
	}

	fn enable_shadows() -> bool {
		false
	}

	fn specialize(
		_pipeline: &MaterialPipeline,
		descriptor: &mut RenderPipelineDescriptor,
		_layout: &MeshVertexBufferLayoutRef,
		_key: MaterialPipelineKey<Self>,
	) -> Result<(), SpecializedMeshPipelineError> {
		descriptor.primitive.cull_mode = Some(Face::Back);
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
		}
		Ok(())
	}
}

impl NearestNeighborMaterial {
	pub fn new(source: Handle<Image>, source_size: UVec2, target_size: UVec2) -> Self {
		Self {
			source,
			params: TreatmentParams {
				source_size: source_size.as_vec2(),
				target_size: target_size.as_vec2(),
			},
		}
	}
}

/// Bundle of presentation materials for different treatments.
pub enum PresentationMaterial {
	Impressionist(Handle<ImpressionistMaterial>),
	PrePostModern(Handle<PrePostModernMaterial>),
	NearestNeighbor(Handle<NearestNeighborMaterial>),
}

pub struct PresentationMaterialsPlugin;

impl Plugin for PresentationMaterialsPlugin {
	fn build(&self, app: &mut App) {
		embedded_asset!(app, "presentation.wgsl");
		app.add_plugins((
			MaterialPlugin::<ImpressionistMaterial>::default(),
			MaterialPlugin::<PrePostModernMaterial>::default(),
			MaterialPlugin::<NearestNeighborMaterial>::default(),
		));
	}
}

pub fn create_presentation_material(
	treatment: PresentationTreatment,
	source: Handle<Image>,
	source_size: UVec2,
	target_size: UVec2,
	impressionist: &mut Assets<ImpressionistMaterial>,
	pre_post_modern: &mut Assets<PrePostModernMaterial>,
	nearest_neighbor: &mut Assets<NearestNeighborMaterial>,
) -> PresentationMaterial {
	match treatment {
		PresentationTreatment::Impressionist => {
			let material = ImpressionistMaterial::new(source, source_size, target_size);
			PresentationMaterial::Impressionist(impressionist.add(material))
		}
		PresentationTreatment::PrePostModern => {
			let material = PrePostModernMaterial::new(source, source_size, target_size);
			PresentationMaterial::PrePostModern(pre_post_modern.add(material))
		}
		PresentationTreatment::NearestNeighbor => {
			let material = NearestNeighborMaterial::new(source, source_size, target_size);
			PresentationMaterial::NearestNeighbor(nearest_neighbor.add(material))
		}
		PresentationTreatment::None => {
			panic!("create_presentation_material called with PresentationTreatment::None")
		}
	}
}
