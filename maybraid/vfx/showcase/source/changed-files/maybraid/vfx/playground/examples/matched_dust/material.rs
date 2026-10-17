use bevy::{
	mesh::MeshVertexBufferLayoutRef,
	pbr::{MaterialPipeline, MaterialPipelineKey},
	prelude::*,
	render::render_resource::*,
	shader::ShaderRef,
};
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct Playback {
	pub time: Vec4,      // i,j,blend,mode(0=B,1=C,2=B+)
	pub settings: Vec4,  // group count, ray steps, sigmaT, active
	pub influence: Vec4, // world xyz, smoothed movement strength
	pub velocity: Vec4,  // xyz, radius
	pub origins: [Vec4; 10],
	pub right: Vec4,
	pub up: Vec4,
	pub centre: Vec4,
	pub experiment: Vec4, // nearest-view flag, optical density multiplier, boundary exclusion, reserved
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DustPlaybackMaterial {
	#[uniform(0)]
	pub playback: Playback,
	#[texture(1)]
	#[sampler(2)]
	pub atlas: Handle<Image>,
	#[texture(3, dimension = "3d")]
	#[sampler(4)]
	pub density: Handle<Image>,
	#[texture(5, dimension = "2d_array")]
	#[sampler(6)]
	pub multiview: Handle<Image>,
}
impl Material for DustPlaybackMaterial {
	fn vertex_shader() -> ShaderRef {
		"demo_shaders://shader.wgsl".into()
	}
	fn fragment_shader() -> ShaderRef {
		"demo_shaders://shader.wgsl".into()
	}
	fn alpha_mode(&self) -> AlphaMode {
		AlphaMode::Blend
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
		descriptor.primitive.cull_mode = None;
		if let Some(depth) = descriptor.depth_stencil.as_mut() {
			depth.depth_write_enabled = Some(false);
			depth.depth_compare = Some(CompareFunction::Always);
		}
		Ok(())
	}
}
